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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn reseed(&mut self, seed: u32) {
        self.white.reseed(seed);
        self.prev = self.white.next();
    }
}

// ── Switchable Noise Source ─────────────────────────────────────────────────

/// Unified noise generator that selects between white, pink, brown or blue.
/// Keeps all variants inline so switching types is allocation-free and safe
/// for the real-time audio thread.
#[derive(Clone, Copy, Debug)]
pub enum NoiseSource {
    White(WhiteNoise),
    Pink(PinkNoise),
    Brown(BrownNoise),
    Blue(BlueNoise),
}

impl NoiseSource {
    /// Create a noise source of the requested colour.
    /// `noise_type`: 0 = white, 1 = pink, 2 = brown, 3 = blue.
    pub fn new(noise_type: u8, seed: u32) -> Self {
        match noise_type {
            1 => NoiseSource::Pink(PinkNoise::new(seed)),
            2 => NoiseSource::Brown(BrownNoise::new(seed)),
            3 => NoiseSource::Blue(BlueNoise::new(seed)),
            _ => NoiseSource::White(WhiteNoise::new(seed)),
        }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        match self {
            NoiseSource::White(n) => n.next(),
            NoiseSource::Pink(n) => n.next(),
            NoiseSource::Brown(n) => n.next(),
            NoiseSource::Blue(n) => n.next(),
        }
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

    /// Configure as a 2-pole low-pass (RBJ cookbook). `q` sets the resonance
    /// (0.707 ≈ Butterworth, higher = a peak at the cutoff).
    pub fn set_lowpass(&mut self, freq: f32, q: f32, sample_rate: f32) {
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let q = q.max(0.1);
        let w0 = 2.0 * std::f32::consts::PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 - cos_w0) * 0.5) / a0;
        self.b1 = (1.0 - cos_w0) / a0;
        self.b2 = ((1.0 - cos_w0) * 0.5) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Configure as a 2-pole high-pass (RBJ cookbook).
    pub fn set_highpass(&mut self, freq: f32, q: f32, sample_rate: f32) {
        let f = freq.clamp(10.0, sample_rate * 0.45);
        let q = q.max(0.1);
        let w0 = 2.0 * std::f32::consts::PI * f / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 + cos_w0) * 0.5) / a0;
        self.b1 = -(1.0 + cos_w0) / a0;
        self.b2 = ((1.0 + cos_w0) * 0.5) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
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

    #[allow(dead_code)] // reusable primitive; amp voices now use DecayReleaseEnvelope
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
    #[allow(dead_code)] // reusable primitive; amp voices now use DecayReleaseEnvelope
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
    #[allow(dead_code)] // reusable primitive; superseded by the [179] retrigger
                        // contract (every hit restarts from a clean slate)
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

    /// Hard machine-gun retrigger: snap to 0, then full attack ramp to peak.
    /// Unlike `trigger_at_peak` which ramps from the current value (smooth
    /// roll), this always restarts the attack from silence.
    pub fn trigger_from_zero(&mut self, peak: f32) {
        let peak = peak.max(0.0);
        self.value = 0.0;
        self.attack_peak = peak;
        self.hold_remaining = self.hold_time;
        if self.attack_time > 0.0 {
            self.attack_start_value = 0.0;
            self.attack_remaining = self.attack_time;
        } else {
            self.value = peak;
        }
    }

    /// Returns the envelope's current value without ticking the decay. Useful for
    /// chaining envelopes that need to observe each other's state at trigger time.
    #[allow(dead_code)] // reusable primitive; amp voices now use DecayReleaseEnvelope
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
/// Bipolar curve shaping of a normalised progress `e` in [0,1].
/// `curve` -1 = concave (slow → fast), 0 = linear, +1 = convex (fast → slow).
/// Exponent 1+5|c| ([170] — pushed from 3|c| for more extreme shapes).
#[inline]
pub fn shape_curve(e: f32, curve: f32) -> f32 {
    let e = e.clamp(0.0, 1.0);
    let c = curve.clamp(-1.0, 1.0);
    if c >= 0.0 {
        e.powf(1.0 + c * 5.0)
    } else {
        1.0 - (1.0 - e).powf(1.0 - c * 5.0)
    }
}

/// Amplitude envelope: **Attack-Hold-Decay** with independent BIPOLAR curve
/// shaping on the attack and the decay (no release stage). Time-based; the
/// attack ramps from the value captured at trigger time so retriggering during
/// a ringing tail never jumps in one sample (anti-click).
///
/// API is kept identical to the previous decay+release version so the voices
/// don't change: the `decay_curve` param/`set_decay_curve` is the bipolar DECAY
/// curve, the `release_curve` param/`set_release_curve` is repurposed as the
/// bipolar ATTACK curve, and `release_time`/`set_release` are ignored.
#[derive(Clone, Copy, Debug)]
pub struct DecayReleaseEnvelope {
    sample_rate: f32,
    attack_time: f32,
    atk_curve: f32,
    hold_time: f32,
    decay_time: f32,
    dec_curve: f32,
    value: f32,
    time: f32,
    /// Value captured at trigger time — the attack ramps up from here.
    attack_start_value: f32,
    threshold: f32,
    active: bool,
}

impl DecayReleaseEnvelope {
    const MIN_ATTACK_S: f32 = 0.0001;

    /// `decay_curve` = bipolar decay curve, `release_curve` = bipolar attack
    /// curve, `_release_time` ignored.
    pub fn new(
        sample_rate: f32,
        decay_curve: f32,
        decay_time: f32,
        release_curve: f32,
        _release_time: f32,
    ) -> Self {
        Self {
            sample_rate,
            attack_time: Self::MIN_ATTACK_S,
            atk_curve: release_curve,
            hold_time: 0.0,
            decay_time: decay_time.max(0.001),
            dec_curve: decay_curve,
            value: 0.0,
            time: 1.0e9, // idle until first trigger
            attack_start_value: 0.0,
            threshold: 0.001,
            active: false,
        }
    }

    pub fn with_attack_ms(mut self, ms: f32) -> Self {
        self.attack_time = (ms.max(0.0) / 1000.0).max(Self::MIN_ATTACK_S);
        self
    }

    pub fn set_attack_ms(&mut self, ms: f32) {
        self.attack_time = (ms.max(0.0) / 1000.0).max(Self::MIN_ATTACK_S);
    }

    /// Retrigger ramping from the current value (anti-click on a ringing tail).
    pub fn trigger(&mut self) {
        self.attack_start_value = self.value.clamp(0.0, 1.0);
        self.time = 0.0;
        self.active = true;
    }

    /// Machine-gun retrigger: restart the whole envelope from zero.
    pub fn trigger_hard(&mut self) {
        self.attack_start_value = 0.0;
        self.value = 0.0;
        self.time = 0.0;
        self.active = true;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        // Advance first so the first sample after trigger() is one step into the
        // ramp (strictly > 0), never the start point itself.
        self.time += 1.0 / self.sample_rate;
        let a = self.attack_time.max(Self::MIN_ATTACK_S);
        let h = self.hold_time.max(0.0);
        let d = self.decay_time.max(0.001);
        let t = self.time;
        let v = if t < a {
            let p = shape_curve(t / a, self.atk_curve);
            self.attack_start_value + (1.0 - self.attack_start_value) * p
        } else if t < a + h {
            1.0
        } else {
            let p = ((t - a - h) / d).clamp(0.0, 1.0);
            shape_curve(1.0 - p, self.dec_curve)
        };
        let v = v.clamp(0.0, 1.0);
        // Only end during the decay tail (never during attack/hold).
        if t >= a + h && v <= self.threshold {
            self.value = 0.0;
            self.active = false;
            return 0.0;
        }
        self.value = v;
        v
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
        self.time = 1.0e9;
        self.attack_start_value = 0.0;
        self.active = false;
    }

    pub fn set_decay(&mut self, decay_time: f32) {
        self.decay_time = decay_time.max(0.001);
    }

    /// No-op — the envelope has no release stage anymore.
    pub fn set_release(&mut self, _release_time: f32) {}

    pub fn set_decay_curve(&mut self, curve: f32) {
        self.dec_curve = curve;
    }

    /// Repurposed: sets the bipolar ATTACK curve.
    pub fn set_release_curve(&mut self, curve: f32) {
        self.atk_curve = curve;
    }

    pub fn set_hold(&mut self, hold_seconds: f32) {
        self.hold_time = hold_seconds.max(0.0);
    }

    #[allow(dead_code)]
    pub fn current(&self) -> f32 {
        self.value
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

    /// Update the sweep duration WITHOUT resetting the running envelope
    /// (recreating the envelope in `set_settings` restarts the sweep mid-drag).
    pub fn set_sweep_time(&mut self, sweep_time: f32) {
        self.sweep_time = sweep_time.max(0.001);
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

// -- Retrigger Declicker ----------------------------------------------------

/// Voice-steal declicker for voices that restart their whole state on every hit.
///
/// A voice that keeps its oscillator phase, filter and smoother state across a
/// retrigger produces a DIFFERENT attack depending on the spacing between two
/// steps (measured on the kick, digital mode, identical settings: 3.7 dB of peak
/// spread, an inverted first half-cycle and a time-to-peak wandering between
/// 1.5 ms and 8.3 ms). Restarting everything from a clean slate fixes that
/// (spread -> 0.00 dB) but steps the output from the ringing tail down to zero,
/// which is a click by definition.
///
/// This closes that gap: it captures the last emitted sample and fades it to
/// zero with a raised-cosine window, so the OUTPUT stays continuous while the
/// new hit starts from scratch. Measured worst sample-to-sample step at the
/// retrigger: 0.014, i.e. 4x cleaner than the phase-continuous retrigger it
/// replaces (0.058).
#[derive(Clone, Copy, Debug)]
pub struct RetrigDeclick {
    /// Value captured at trigger time — the fade starts from here.
    start: f32,
    /// Remaining fade samples (0 = idle).
    remaining: u32,
    /// Total fade length in samples.
    length: u32,
}

impl RetrigDeclick {
    /// Fade length. 3 ms measured as the sweet spot: shorter leaves an audible
    /// step, longer bleeds the old tail into the new attack.
    pub const FADE_MS: f32 = 3.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            start: 0.0,
            remaining: 0,
            length: ((sample_rate * Self::FADE_MS / 1000.0) as u32).max(1),
        }
    }

    /// Arm the fade from `last_out`, the voice's last emitted sample. Call at the
    /// top of `trigger()`, BEFORE resetting the voice state.
    pub fn arm(&mut self, last_out: f32) {
        self.start = last_out;
        self.remaining = self.length;
    }

    /// Next fade sample. ADD it to the voice's final output — post saturation and
    /// post volume — so it continues the exact sample it was captured from.
    #[inline]
    pub fn next(&mut self) -> f32 {
        if self.remaining == 0 {
            return 0.0;
        }
        // First call returns `start` unchanged (t = 0, window = 1), so the output
        // is continuous across the trigger sample.
        let t = (self.length - self.remaining) as f32 / self.length as f32;
        self.remaining -= 1;
        let window = 0.5 * (1.0 + (std::f32::consts::PI * t).cos());
        self.start * window
    }

    pub fn is_active(&self) -> bool {
        self.remaining > 0
    }

    pub fn reset(&mut self) {
        self.start = 0.0;
        self.remaining = 0;
    }
}

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

// ── Analog Drift ─────────────────────────────────────────────────────────────

/// Per-hit "analog" drift: on each trigger it pulls small random multipliers so
/// no two hits are identical (the vintage "breathing"). In digital mode all
/// factors stay exactly 1.0 (bit-identical hits). Click-safe: the factors scale a
/// frequency (phase-accumulator → no phase jump), a gain, and envelope times —
/// none of which introduce a discontinuity.
#[derive(Clone, Copy, Debug)]
pub struct AnalogDrift {
    rng: WhiteNoise,
    /// Multiplier on oscillator frequency (detune).
    pub pitch: f32,
    /// Multiplier on output level.
    pub level: f32,
    /// Multiplier on envelope decay/release times (tail length).
    pub time: f32,
}

impl AnalogDrift {
    /// Drift depths, shared across voices so the analog character is consistent.
    /// These are the MAXIMUM bounds (±). Increase them if the analog effect feels
    /// too subtle; decrease if it feels too extreme.
    /// Calibrated for a clearly audible "breathing" on a dense 16-step pattern.
    pub const PITCH_DEPTH: f32 = 0.075; // ±7.5 % detune (~130 cents)
    pub const LEVEL_DEPTH: f32 = 0.25; // ±25 % level (~2 dB)
    pub const TIME_DEPTH: f32 = 0.50; // ±50 % envelope time (tail length)

    pub fn new(seed: u32) -> Self {
        Self {
            rng: WhiteNoise::new(seed),
            pitch: 1.0,
            level: 1.0,
            time: 1.0,
        }
    }

    /// Recompute the drift factors for a new hit. `analog == true` → random
    /// offsets; `false` → all 1.0 (deterministic, bit-identical hits).
    #[inline]
    pub fn trigger(&mut self, analog: bool) {
        if analog {
            self.pitch = 1.0 + self.rng.next() * Self::PITCH_DEPTH;
            self.level = 1.0 + self.rng.next() * Self::LEVEL_DEPTH;
            self.time = 1.0 + self.rng.next() * Self::TIME_DEPTH;
        } else {
            self.pitch = 1.0;
            self.level = 1.0;
            self.time = 1.0;
        }
    }
}

/// Per-hit drift for tone, level, and timing: modulates the "tone"
/// parameter (peaking center, oscillator base, or filter cutoff), output level,
/// and envelope timing by small random amounts on each trigger. The depth scales
/// continuously with the `analog` parameter so 0.0 is deterministic and 1.0 is
/// maximum drift.
#[derive(Clone, Copy, Debug)]
pub struct ToneDrift {
    rng: WhiteNoise,
    /// Maximum relative drift per hit (e.g. 0.075 = ±7.5 %).
    depth: f32,
    /// Current frequency multiplier applied to the tone parameter.
    pub multiplier: f32,
    /// Current level multiplier applied to the output volume.
    pub level_multiplier: f32,
    /// Current timing offset in seconds (±2 ms max).
    pub timing_offset: f32,
}

impl ToneDrift {
    pub fn new(seed: u32, depth: f32) -> Self {
        Self {
            rng: WhiteNoise::new(seed),
            depth,
            multiplier: 1.0,
            level_multiplier: 1.0,
            timing_offset: 0.0,
        }
    }

    /// Recompute the drift factors for a new hit. `analog` is the user
    /// parameter in [0, 1]; 0 disables drift, 1 applies full `depth`.
    #[inline]
    pub fn trigger(&mut self, analog: f32) {
        let amount = analog.clamp(0.0, 1.0);
        if amount == 0.0 {
            self.multiplier = 1.0;
            self.level_multiplier = 1.0;
            self.timing_offset = 0.0;
        } else {
            self.multiplier = 1.0 + self.rng.next() * self.depth * amount;
            self.level_multiplier = 1.0 + self.rng.next() * 0.1 * amount; // ±10 % level
            self.timing_offset = self.rng.next() * 0.002 * amount; // ±2 ms timing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_drift_is_deterministic_at_zero_and_varies_at_full() {
        let depth = 0.075;
        let mut drift = ToneDrift::new(12345, depth);

        // analog = 0 must always give a multiplier of exactly 1.0.
        drift.trigger(0.0);
        assert!(
            (drift.multiplier - 1.0).abs() < 1e-6,
            "analog=0 should disable tone drift: {}",
            drift.multiplier
        );

        // analog = 1 should give a non-trivial drift within the declared depth.
        drift.trigger(1.0);
        assert!(
            (drift.multiplier - 1.0).abs() > 1e-4,
            "analog=1 should produce audible tone drift: {}",
            drift.multiplier
        );
        assert!(
            (drift.multiplier - 1.0).abs() <= depth,
            "drift should not exceed depth: {}",
            drift.multiplier
        );

        // Intermediate amounts should scale proportionally.
        let mut small = ToneDrift::new(12345, depth);
        small.trigger(0.5);
        assert!(
            (small.multiplier - 1.0).abs() <= depth * 0.5 + 1e-6,
            "analog=0.5 should scale drift by 0.5: {}",
            small.multiplier
        );
    }

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
