//! Shared DSP primitives for drum synthesis.
//!
//! All structs are designed for sample-by-sample, allocation-free real-time use.

// ── White Noise ─────────────────────────────────────────────────────────────

/// Deterministic white-noise generator (XORShift LCG).
#[derive(Clone, Copy, Debug)]
pub struct WhiteNoise {
    state: u32,
}

impl WhiteNoise {
    pub fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        // Map to [-1.0, 1.0]
        (self.state as f32 / 2147483648.0) - 1.0
    }

    pub fn reseed(&mut self, seed: u32) {
        self.state = seed.max(1);
    }
}

// ── Pink Noise (Voss-McCartney approximation) ───────────────────────────────

/// Deterministic pink-noise generator using the Voss-McCartney algorithm
/// with 8 rows of random values and a running sum.
#[derive(Clone, Copy, Debug)]
pub struct PinkNoise {
    rows: [f32; 8],
    index: u8,
    white: WhiteNoise,
}

impl PinkNoise {
    pub fn new(seed: u32) -> Self {
        let mut white = WhiteNoise::new(seed);
        Self {
            rows: std::array::from_fn(|_| white.next()),
            index: 0,
            white,
        }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        // Count trailing zeros of index to pick which row to update
        let tz = (self.index.trailing_zeros() as usize).min(7);
        self.rows[tz] = self.white.next();
        self.index = self.index.wrapping_add(1);
        let sum: f32 = self.rows.iter().sum();
        // Normalise approximativement vers [-1, 1]
        sum * 0.25
    }

    pub fn reseed(&mut self, seed: u32) {
        self.white.reseed(seed);
        for r in self.rows.iter_mut() {
            *r = self.white.next();
        }
        self.index = 0;
    }
}

// ── Brown Noise (1/f², integration of white) ────────────────────────────────

/// Brown/red noise: integration of white noise with a gentle leak.
#[derive(Clone, Copy, Debug)]
pub struct BrownNoise {
    integrator: f32,
    white: WhiteNoise,
}

impl BrownNoise {
    pub fn new(seed: u32) -> Self {
        Self {
            integrator: 0.0,
            white: WhiteNoise::new(seed),
        }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        self.integrator += self.white.next() * 0.12;
        // Gentle leak to prevent DC runaway
        self.integrator *= 0.995;
        self.integrator
    }

    pub fn reseed(&mut self, seed: u32) {
        self.white.reseed(seed);
        self.integrator = 0.0;
    }
}

// ── Blue Noise (+3 dB/octave, differentiation of white) ─────────────────────

/// Blue/violet noise: differentiation of white noise.
#[derive(Clone, Copy, Debug)]
pub struct BlueNoise {
    prev: f32,
    white: WhiteNoise,
}

impl BlueNoise {
    pub fn new(seed: u32) -> Self {
        let mut white = WhiteNoise::new(seed);
        let first = white.next();
        Self { prev: first, white }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let current = self.white.next();
        let diff = current - self.prev;
        self.prev = current;
        diff * 0.8
    }

    pub fn reseed(&mut self, seed: u32) {
        self.white.reseed(seed);
        self.prev = self.white.next();
    }
}

// ── One-Pole Filter ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterMode {
    LowPass,
    HighPass,
}

/// First-order one-pole IIR filter (6 dB/octave).
#[derive(Clone, Copy, Debug)]
pub struct OnePoleFilter {
    state: f32,
    alpha: f32,
    mode: FilterMode,
}

impl OnePoleFilter {
    pub fn new(mode: FilterMode) -> Self {
        Self {
            state: 0.0,
            alpha: 1.0,
            mode,
        }
    }

    /// Set cutoff frequency. `sample_rate` in Hz.
    pub fn set_cutoff(&mut self, freq: f32, sample_rate: f32) {
        // Prevent division by zero and Nyquist overflow
        let f = freq.clamp(10.0, sample_rate * 0.49);
        self.alpha = 1.0 - (-2.0 * std::f32::consts::PI * f / sample_rate).exp();
        self.alpha = self.alpha.clamp(0.0001, 1.0);
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        match self.mode {
            FilterMode::LowPass => {
                self.state += self.alpha * (input - self.state);
                self.state
            }
            FilterMode::HighPass => {
                self.state += self.alpha * (input - self.state);
                input - self.state
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = 0.0;
    }
}

// ── Biquad ──────────────────────────────────────────────────────────────────

/// Direct-form-I biquad. Used for resonant bandpass filters (e.g. the
/// bridged-T resonator that gives the TR-606/808 toms and snare their pitched
/// "body"). Two poles → 12 dB/octave with adjustable resonance.
#[derive(Clone, Copy, Debug)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    pub fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Configure as a constant-skirt bandpass (RBJ cookbook). `freq` in Hz,
    /// `q` is the resonance — higher Q = narrower / more ringing.
    pub fn set_bandpass(&mut self, freq: f32, q: f32, sample_rate: f32) {
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let q = q.max(0.1);
        let w0 = 2.0 * std::f32::consts::PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b1 = 0.0;
        self.b2 = -alpha / a0;
        self.a1 = -2.0 * cos_w0 / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Configure as a peaking EQ (RBJ cookbook). `freq` in Hz, `q` is the
    /// bandwidth, `gain_db` is the boost/cut in dB (positive = boost).
    pub fn set_peaking(&mut self, freq: f32, q: f32, gain_db: f32, sample_rate: f32) {
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let q = q.max(0.1);
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        let a0 = 1.0 + alpha / a;
        self.b0 = (1.0 + alpha * a) / a0;
        self.b1 = (-2.0 * cos_w0) / a0;
        self.b2 = (1.0 - alpha * a) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha / a) / a0;
    }

    #[inline]
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

// ── Exponential Decay Envelope ──────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct ExpDecayEnvelope {
    /// Current envelope value. Free to exceed 1.0 when driven by
    /// `trigger_from_current(peak)` with a physical unit (delta-Hz, gain reduction).
    value: f32,
    /// Per-sample decay coefficient: `value(n+1) = value(n) * coeff`.
    /// Equivalent to `exp(-curve * t / decay_time)` but evaluated recursively, so
    /// no `exp()` is called on the audio thread.
    coeff: f32,
    /// Sample rate, kept for `set_decay` and to derive the attack ramp duration.
    sample_rate: f32,
    /// Decay time constant: `t` such that `value(t) ≈ exp(-curve)`.
    decay_time: f32,
    /// Steepness multiplier. Higher = faster fall (same semantics as before).
    curve: f32,
    /// Threshold below which the value is snapped to 0 (silence detection).
    threshold: f32,
    /// Anti-click attack ramp duration in seconds. Mimics the RC charge time of an
    /// analog VCA so retriggering during a ringing tail interpolates from the
    /// current value to the target peak instead of jumping.
    attack_time: f32,
    /// Remaining attack ramp time in seconds. While > 0 the envelope is in
    /// attack phase.
    attack_remaining: f32,
    /// Envelope value captured at the moment of trigger; ramp start point.
    attack_start_value: f32,
    /// Target the attack ramp climbs to (1.0 by default, smaller for stages
    /// triggered via `trigger_at_peak`).
    attack_peak: f32,
    /// Hold time in seconds. After the attack ramp completes, the envelope
    /// stays at `attack_peak` for this many seconds before decay starts.
    /// 0 = no hold (AD shape); > 0 = AHD shape.
    hold_time: f32,
    /// Remaining hold time in seconds. While > 0 the envelope stays at peak.
    hold_remaining: f32,
}

impl ExpDecayEnvelope {
    pub fn new(sample_rate: f32, curve: f32, decay_time: f32) -> Self {
        let mut env = Self {
            value: 0.0,
            coeff: 1.0,
            sample_rate,
            decay_time: decay_time.max(0.001),
            curve,
            threshold: 0.001,
            attack_time: 0.0,
            attack_remaining: 0.0,
            attack_start_value: 0.0,
            attack_peak: 1.0,
            hold_time: 0.0,
            hold_remaining: 0.0,
        };
        env.recompute_coeff();
        env
    }

    fn recompute_coeff(&mut self) {
        let dt = 1.0 / self.sample_rate;
        self.coeff = (-self.curve * dt / self.decay_time).exp();
    }

    /// Configure a short attack ramp (in milliseconds) applied on every trigger.
    /// Set to 0 to keep the original instantaneous jump behavior.
    pub fn with_attack_ms(mut self, ms: f32) -> Self {
        self.attack_time = ms.max(0.0) / 1000.0;
        self
    }

    pub fn set_attack_ms(&mut self, ms: f32) {
        self.attack_time = ms.max(0.0) / 1000.0;
        // If attack was shortened to zero while a ramp is still in progress,
        // snap immediately to peak so we never divide by zero in next().
        if self.attack_time == 0.0 && self.attack_remaining > 0.0 {
            self.attack_remaining = 0.0;
            self.value = self.attack_peak;
        }
    }

    /// Set the hold time in seconds. After the attack ramp completes, the
    /// envelope stays at its peak for `hold_seconds` before the decay starts.
    pub fn set_hold(&mut self, hold_seconds: f32) {
        self.hold_time = hold_seconds.max(0.0);
    }

    pub fn trigger(&mut self) {
        self.trigger_at_peak(1.0);
    }

    /// Trigger the envelope toward a custom peak instead of the default 1.0.
    /// Respects the attack ramp configured via `with_attack_ms`. Used by the
    /// release stage of `DecayReleaseEnvelope` so its shelf level rises
    /// gradually instead of jumping in one sample (which was the source of an
    /// audible click at every retrigger).
    ///
    /// If the current value is already at or above `peak`, no ramp is fired —
    /// the natural decay carries the envelope back. This preserves persistent
    /// retrigger behaviour for tails that are still louder than the new peak.
    pub fn trigger_at_peak(&mut self, peak: f32) {
        let peak = peak.max(0.0);
        self.attack_peak = peak;
        // Reset hold counter — fires after the attack ramp completes (or
        // immediately if no attack ramp is configured).
        self.hold_remaining = self.hold_time;
        if self.value >= peak {
            self.attack_remaining = 0.0;
            return;
        }
        if self.attack_time > 0.0 {
            self.attack_start_value = self.value;
            self.attack_remaining = self.attack_time;
        } else {
            self.value = peak;
        }
    }

    /// Analog-style persistent retrigger: bump the envelope to `peak` only if it
    /// is currently below that value, otherwise keep the existing tail. Bypasses
    /// the attack ramp — intended for usages where the value carries a physical
    /// quantity (delta-Hz for a pitch sweep) rather than an amplitude.
    pub fn trigger_from_current(&mut self, peak: f32) {
        let peak = peak.max(0.0);
        self.value = self.value.max(peak);
        self.attack_remaining = 0.0;
    }

    /// Deterministic retrigger: snap the value to exactly `value`, no attack ramp.
    /// Unlike `trigger_from_current` (which only ever raises the value, giving the
    /// organic "analog" drift), this restarts from the same depth on every hit —
    /// used by the "digital" path so the pitch sweep is identical each time.
    /// Click-safe because the value drives a *frequency* (phase-accumulator), so a
    /// jump changes the phase slope, never the phase itself.
    pub fn trigger_reset_to(&mut self, value: f32) {
        self.value = value.max(0.0);
        self.attack_remaining = 0.0;
        self.hold_remaining = self.hold_time;
    }

    /// Returns the envelope's current value without ticking the decay. Useful for
    /// chaining envelopes that need to observe each other's state at trigger time.
    pub fn current(&self) -> f32 {
        self.value
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let dt = 1.0 / self.sample_rate;

        if self.attack_remaining > 0.0 {
            // Decrement first so the first sample after trigger() is one ramp step
            // in, not the start point itself. That way `env.next()` is always
            // strictly > 0 (assuming attack_start_value < 1) and reaches exactly 1.0
            // at the final attack sample.
            self.attack_remaining -= dt;
            if self.attack_remaining <= 0.0 {
                self.attack_remaining = 0.0;
                self.value = self.attack_peak;
                return self.value;
            }
            let t = 1.0 - (self.attack_remaining / self.attack_time);
            self.value = self.attack_start_value + (self.attack_peak - self.attack_start_value) * t;
            return self.value;
        }

        if self.hold_remaining > 0.0 {
            // Hold phase: value stays at the attack peak until the hold time
            // elapses. Once it does, the recursive decay below takes over.
            self.hold_remaining -= dt;
            return self.value;
        }

        // Recursive exponential decay — no per-sample exp() call.
        let out = self.value;
        self.value *= self.coeff;
        if self.value < self.threshold {
            self.value = 0.0;
        }
        out
    }

    pub fn is_active(&self) -> bool {
        self.attack_remaining > 0.0 || self.hold_remaining > 0.0 || self.value > self.threshold
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
        self.attack_remaining = 0.0;
        self.attack_start_value = 0.0;
        self.hold_remaining = 0.0;
    }

    pub fn set_decay(&mut self, decay_time: f32) {
        self.decay_time = decay_time.max(0.001);
        self.recompute_coeff();
    }

    pub fn set_curve(&mut self, curve: f32) {
        self.curve = curve.max(0.1);
        self.recompute_coeff();
    }
}

// ── Decay+Release Envelope ──────────────────────────────────────────────────

/// Two-stage amplitude envelope: a fast `decay` falling from 1.0 with steep
/// curve, and a slow `release` plateauing at a fixed shelf (~30 % of peak) then
/// dropping to 0 with a long time constant.
///
/// The output is `max(decay.next(), release.next())` so the two stages cross
/// over smoothly: the decay dominates the punch, then the release takes over
/// once the decay falls below the shelf. No amplitude boost at trigger time —
/// peak is always 1.0.
///
/// Both internal envelopes are persistent on retrigger
/// (`trigger_from_current`), which preserves continuity when a step lands
/// during a ringing tail.
#[derive(Clone, Copy, Debug)]
pub struct DecayReleaseEnvelope {
    decay: ExpDecayEnvelope,
    release: ExpDecayEnvelope,
    /// Shelf level the release envelope plateaus at (and decays from).
    /// 0.3 = release takes over once the decay has fallen to 30 % of peak.
    release_shelf: f32,
}

impl DecayReleaseEnvelope {
    /// Minimum release time. A value of 0 from the UI is clamped to this so
    /// the recursive coefficient stays well-defined; the resulting tail is
    /// only a handful of samples, perceptually equivalent to "no release".
    pub const MIN_RELEASE_SECONDS: f32 = 0.001;
    /// Default shelf level — tuned so 808-style sub tails sound natural.
    pub const DEFAULT_RELEASE_SHELF: f32 = 0.3;

    pub fn new(
        sample_rate: f32,
        decay_curve: f32,
        decay_time: f32,
        release_curve: f32,
        release_time: f32,
    ) -> Self {
        Self {
            decay: ExpDecayEnvelope::new(sample_rate, decay_curve, decay_time),
            release: ExpDecayEnvelope::new(
                sample_rate,
                release_curve,
                release_time.max(Self::MIN_RELEASE_SECONDS),
            ),
            release_shelf: Self::DEFAULT_RELEASE_SHELF,
        }
    }

    pub fn with_attack_ms(mut self, ms: f32) -> Self {
        // Apply the same attack ramp to both stages so the release shelf rises
        // smoothly toward its target instead of jumping in one sample — that
        // jump (0 → 0.3 at every trigger) was creating an audible click on
        // cold starts independent of any tail ringing.
        self.decay = self.decay.with_attack_ms(ms);
        self.release = self.release.with_attack_ms(ms);
        self
    }

    pub fn set_attack_ms(&mut self, ms: f32) {
        self.decay.set_attack_ms(ms);
        self.release.set_attack_ms(ms);
    }

    pub fn trigger(&mut self) {
        self.decay.trigger();
        // Release stage ramps from its current value up to the shelf level
        // through the attack ramp. If a tail is already ringing above the
        // shelf, trigger_at_peak keeps it (attack_start_value = current value)
        // and the ramp will pull it back toward the shelf.
        self.release.trigger_at_peak(self.release_shelf);
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let d = self.decay.next();
        let r = self.release.next();
        if d > r {
            d
        } else {
            r
        }
    }

    pub fn is_active(&self) -> bool {
        self.decay.is_active() || self.release.is_active()
    }

    pub fn reset(&mut self) {
        self.decay.reset();
        self.release.reset();
    }

    pub fn set_decay(&mut self, decay_time: f32) {
        self.decay.set_decay(decay_time);
    }

    pub fn set_release(&mut self, release_time: f32) {
        self.release
            .set_decay(release_time.max(Self::MIN_RELEASE_SECONDS));
    }

    pub fn set_decay_curve(&mut self, curve: f32) {
        self.decay.set_curve(curve);
    }

    pub fn set_release_curve(&mut self, curve: f32) {
        self.release.set_curve(curve);
    }

    /// Set the hold time (seconds) on the decay stage only — the release stage
    /// has no hold semantics. The envelope output stays at peak for
    /// `hold_seconds` after the attack ramp before the decay starts.
    pub fn set_hold(&mut self, hold_seconds: f32) {
        self.decay.set_hold(hold_seconds);
    }

    #[allow(dead_code)]
    pub fn current(&self) -> f32 {
        self.decay.current().max(self.release.current())
    }
}

// ── Pitch Envelope ──────────────────────────────────────────────────────────

/// Exponential pitch sweep from start_ratio → end_ratio over sweep_time seconds.
#[derive(Clone, Copy, Debug)]
pub struct PitchEnvelope {
    current_ratio: f32,
    start_ratio: f32,
    end_ratio: f32,
    sweep_time: f32,
    sample_rate: f32,
    time: f32,
    active: bool,
}

impl PitchEnvelope {
    pub fn new(sample_rate: f32, start_ratio: f32, end_ratio: f32, sweep_time: f32) -> Self {
        Self {
            current_ratio: start_ratio,
            start_ratio,
            end_ratio,
            sweep_time: sweep_time.max(0.001),
            sample_rate,
            time: 0.0,
            active: false,
        }
    }

    pub fn trigger(&mut self) {
        self.current_ratio = self.start_ratio;
        self.time = 0.0;
        self.active = true;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        if !self.active {
            return self.end_ratio;
        }
        self.time += 1.0 / self.sample_rate;
        if self.time >= self.sweep_time {
            self.active = false;
            self.end_ratio
        } else {
            let t = self.time / self.sweep_time;
            // Exponential interpolation
            let ratio = self.start_ratio * (self.end_ratio / self.start_ratio).powf(t);
            self.current_ratio = ratio;
            ratio
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

// ── Click / Transient Generator ─────────────────────────────────────────────

/// Very short impulse + noise burst for attack transients (kick click, stick attack, etc.)
#[derive(Clone, Copy, Debug)]
pub struct ClickGenerator {
    noise: WhiteNoise,
    envelope: ExpDecayEnvelope,
    noise_mix: f32,
    level: f32,
    triggered: bool,
    /// Samples elapsed since the most recent trigger. Drives the single-sample
    /// impulse at the start of the click.
    samples_since_trigger: u32,
}

impl ClickGenerator {
    pub fn new(sample_rate: f32, decay_ms: f32, noise_mix: f32, level: f32) -> Self {
        let decay_time = decay_ms / 1000.0;
        Self {
            noise: WhiteNoise::new(0x1234_5678),
            envelope: ExpDecayEnvelope::new(sample_rate, 80.0, decay_time),
            noise_mix: noise_mix.clamp(0.0, 1.0),
            level: level.clamp(0.0, 2.0),
            triggered: false,
            samples_since_trigger: 0,
        }
    }

    pub fn trigger(&mut self) {
        self.triggered = true;
        self.envelope.trigger();
        self.noise.reseed(0x1234_5678);
        self.samples_since_trigger = 0;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        if !self.triggered || !self.envelope.is_active() {
            return 0.0;
        }
        let env = self.envelope.next();
        let impulse = if self.samples_since_trigger == 0 {
            1.0
        } else {
            0.0
        };
        self.samples_since_trigger = self.samples_since_trigger.saturating_add(1);
        let noise = self.noise.next();
        let sample = impulse * (1.0 - self.noise_mix) + noise * self.noise_mix;
        sample * env * self.level
    }

    pub fn is_active(&self) -> bool {
        self.triggered && self.envelope.is_active()
    }

    pub fn reset(&mut self) {
        self.triggered = false;
        self.envelope.reset();
        self.samples_since_trigger = 0;
    }
}

// ── Sine Oscillator ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct SineOsc {
    pub phase: f32,
    phase_increment: f32,
    sample_rate: f32,
}

impl SineOsc {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
        }
    }

    pub fn set_freq(&mut self, freq: f32) {
        self.phase_increment = freq / self.sample_rate;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let sample = (self.phase * 2.0 * std::f32::consts::PI).sin();
        self.phase += self.phase_increment;
        self.phase -= self.phase.floor();
        sample
    }
}

// ── Square Oscillator ───────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct SquareOsc {
    phase: f32,
    phase_increment: f32,
    sample_rate: f32,
}

impl SquareOsc {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
        }
    }

    pub fn set_freq(&mut self, freq: f32) {
        self.phase_increment = freq / self.sample_rate;
    }

    /// Reset phase to zero. Used for cold-start phase alignment only (never on a
    /// retrigger during a ringing tail, which must stay phase-continuous).
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let sample = if self.phase < 0.5 { 1.0 } else { -1.0 };
        self.phase += self.phase_increment;
        self.phase -= self.phase.floor();
        sample
    }
}

// ── Sawtooth Oscillator ─────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct SawOsc {
    pub phase: f32,
    phase_increment: f32,
    sample_rate: f32,
}

impl SawOsc {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
        }
    }

    pub fn set_freq(&mut self, freq: f32) {
        self.phase_increment = freq / self.sample_rate;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let sample = 2.0 * self.phase - 1.0;
        self.phase += self.phase_increment;
        self.phase -= self.phase.floor();
        sample
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

// ── Triangle Oscillator ─────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct TriangleOsc {
    pub phase: f32,
    phase_increment: f32,
    sample_rate: f32,
}

impl TriangleOsc {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
        }
    }

    pub fn set_freq(&mut self, freq: f32) {
        self.phase_increment = freq / self.sample_rate;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        let sample = 2.0 * (self.phase - 0.5).abs() * 2.0 - 1.0;
        self.phase += self.phase_increment;
        self.phase -= self.phase.floor();
        sample
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

// ── One-Pole Smoother ───────────────────────────────────────────────────────

/// First-order smoother for continuous control values (frequency, gain).
/// Internally tracks a target the output is converging towards over `time_ms`.
/// Used in the kick voice to absorb pitch-envelope discontinuities at retrigger
/// time so the oscillator's instantaneous frequency never jumps in one sample.
#[derive(Clone, Copy, Debug)]
pub struct OnePoleSmoother {
    current: f32,
    coeff: f32,
}

impl OnePoleSmoother {
    pub fn new(sample_rate: f32, time_ms: f32, initial: f32) -> Self {
        let mut smoother = Self {
            current: initial,
            coeff: 0.0,
        };
        smoother.set_time_ms(sample_rate, time_ms);
        smoother
    }

    pub fn set_time_ms(&mut self, sample_rate: f32, time_ms: f32) {
        let time_seconds = (time_ms.max(0.01)) * 0.001;
        self.coeff = (-1.0 / (sample_rate * time_seconds)).exp();
    }

    #[allow(dead_code)]
    pub fn reset(&mut self, value: f32) {
        self.current = value;
    }

    #[inline]
    pub fn process(&mut self, target: f32) -> f32 {
        self.current = target + self.coeff * (self.current - target);
        self.current
    }
}

// ── DC Blocker ──────────────────────────────────────────────────────────────

/// One-pole DC blocker: y[n] = x[n] - x[n-1] + r * y[n-1].
/// Removes the slow DC drift that accumulates from asymmetric retriggers and
/// soft-clipping, without colouring the audio band.
#[derive(Clone, Copy, Debug)]
pub struct DcBlocker {
    r: f32,
    x1: f32,
    y1: f32,
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self {
            r: 0.995,
            x1: 0.0,
            y1: 0.0,
        }
    }
}

impl DcBlocker {
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_decay_without_attack_jumps_to_one_on_trigger() {
        let mut env = ExpDecayEnvelope::new(44100.0, 5.0, 0.5);
        env.trigger();
        let first = env.next();
        assert!(
            first > 0.99,
            "Without attack ramp, first sample must be ~1.0: {}",
            first
        );
    }

    #[test]
    fn exp_decay_attack_ramp_avoids_jump_on_retrigger_during_tail() {
        let mut env = ExpDecayEnvelope::new(44100.0, 5.0, 0.5).with_attack_ms(1.5);
        env.trigger();
        // Drain the attack ramp.
        for _ in 0..100 {
            env.next();
        }
        // Decay to mid-range so retrigger sees a ringing tail.
        let mut tail = 0.0;
        for _ in 0..2000 {
            tail = env.next();
        }
        assert!(
            tail > 0.1 && tail < 0.9,
            "Tail must be mid-range for the test to be meaningful: {}",
            tail
        );

        env.trigger();
        let first = env.next();
        let step = (first - tail).abs();
        assert!(
            step < 0.05,
            "Retrigger discontinuity too large: tail={}, first={}, step={}",
            tail,
            first,
            step
        );
    }

    #[test]
    fn exp_decay_attack_ramp_reaches_one_then_decays() {
        let sample_rate = 44100.0;
        let attack_ms = 1.5_f32;
        let mut env = ExpDecayEnvelope::new(sample_rate, 5.0, 0.5).with_attack_ms(attack_ms);
        env.trigger();

        let attack_samples = (attack_ms / 1000.0 * sample_rate).ceil() as usize;
        let mut last = 0.0;
        for _ in 0..attack_samples {
            last = env.next();
        }
        assert!(
            last >= 0.99,
            "Envelope must reach 1.0 by end of attack: {}",
            last
        );

        // Next sample should start the decay (< 1.0 strictly or stay at peak).
        let after_attack = env.next();
        assert!(
            after_attack <= last + 1e-3,
            "Decay should start after attack: last={}, next={}",
            last,
            after_attack
        );
    }
}
