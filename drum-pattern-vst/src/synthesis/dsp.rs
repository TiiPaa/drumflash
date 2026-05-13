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
        Self {
            state: seed.max(1),
        }
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

// ── Exponential Decay Envelope ──────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct ExpDecayEnvelope {
    value: f32,
    /// Decay time constant multiplier. Higher = steeper fall.
    curve: f32,
    /// Target decay time in seconds.
    decay_time: f32,
    /// Sample rate.
    sample_rate: f32,
    /// Threshold below which the voice is considered silent.
    threshold: f32,
    /// Accumulated time in seconds.
    time: f32,
}

impl ExpDecayEnvelope {
    pub fn new(sample_rate: f32, curve: f32, decay_time: f32) -> Self {
        Self {
            value: 0.0,
            curve,
            decay_time,
            sample_rate,
            threshold: 0.001,
            time: 0.0,
        }
    }

    pub fn trigger(&mut self) {
        self.value = 1.0;
        self.time = 0.0;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        self.time += 1.0 / self.sample_rate;
        if self.time >= self.decay_time && self.value <= self.threshold {
            0.0
        } else {
            // Exponential decay: value = exp(-curve * time / decay_time)
            self.value = (-self.curve * self.time / self.decay_time).exp();
            if self.value < self.threshold {
                self.value = 0.0;
            }
            self.value
        }
    }

    pub fn is_active(&self) -> bool {
        self.value > self.threshold || self.time < self.decay_time
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
        self.time = 0.0;
    }

    pub fn set_decay(&mut self, decay_time: f32) {
        self.decay_time = decay_time.max(0.001);
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

    pub fn reset(&mut self) {
        self.active = false;
        self.time = 0.0;
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
        }
    }

    pub fn trigger(&mut self) {
        self.triggered = true;
        self.envelope.trigger();
        self.noise.reseed(0x1234_5678);
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        if !self.triggered || !self.envelope.is_active() {
            return 0.0;
        }
        let env = self.envelope.next();
        let impulse = if self.envelope.time <= 1.0 / self.envelope.sample_rate {
            1.0
        } else {
            0.0
        };
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

    pub fn reset(&mut self) {
        self.phase = 0.0;
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

    #[inline]
    pub fn next(&mut self) -> f32 {
        let sample = if self.phase < 0.5 { 1.0 } else { -1.0 };
        self.phase += self.phase_increment;
        self.phase -= self.phase.floor();
        sample
    }

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
