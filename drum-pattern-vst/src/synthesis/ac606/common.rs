//! Little bits shared by the drum voices.
//!
//! Ported from `SynthDrumCommon.hpp` of analogcode's "606-Inspired-Synth-Drums"
//! — MIT License, Copyright (c) 2026 Matthew Fecher (see `LICENSE-MIT.txt`).

pub const PI: f32 = std::f32::consts::PI;
pub const TWO_PI: f32 = 6.28318530717958647692;
pub const T60_TO_TAU: f32 = 6.9077553;
pub const MINIMUM_STATE_MAGNITUDE: f32 = 1.0e-20;

#[inline]
pub fn flush_denormal(value: f32) -> f32 {
    if value.abs() < MINIMUM_STATE_MAGNITUDE {
        0.0
    } else {
        value
    }
}

#[inline]
pub fn clampf(x: f32, lo: f32, hi: f32) -> f32 {
    x.clamp(lo, hi)
}

#[inline]
pub fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
pub fn soft(x: f32) -> f32 {
    x.tanh()
}

/// Tiny repeatable random numbers. Plenty good enough for drum noise.
pub struct Random {
    state: u32,
}

impl Random {
    pub fn new(seed: u32) -> Self {
        let mut r = Self { state: 0 };
        r.seed(seed);
        r
    }

    pub fn seed(&mut self, s: u32) {
        self.state = if s == 0 { 0x12345678 } else { s };
    }

    pub fn bipolar(&mut self) -> f32 {
        let x = self.next();
        (x as f32) * (2.0 / 4294967295.0) - 1.0
    }

    /// The tom noise was tuned with the top 24 bits of the generator.
    /// Keep that mapping here so the other drum voices do not change.
    pub fn bipolar24(&mut self) -> f32 {
        let x = self.next();
        ((x >> 8) as f32) * (2.0 / 16777215.0) - 1.0
    }

    /// The snare noise was tuned with the bottom 24 bits.
    /// Keep this separate so the tom stream stays exactly the same.
    pub fn bipolar_low24(&mut self) -> f32 {
        let x = self.next();
        ((x & 0x00FF_FFFF) as f32) / 8388608.0 - 1.0
    }

    fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }
}

pub struct DcBlocker {
    x1: f32,
    y1: f32,
}

impl DcBlocker {
    pub fn new() -> Self {
        Self { x1: 0.0, y1: 0.0 }
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + 0.995 * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

pub struct OnePoleLpf {
    sample_rate: f64,
    a: f32,
    z: f32,
}

impl OnePoleLpf {
    pub fn new(sample_rate: f64) -> Self {
        let mut f = Self {
            sample_rate,
            a: 0.1,
            z: 0.0,
        };
        f.set_cutoff_hz(1000.0);
        f
    }

    pub fn reset(&mut self) {
        self.z = 0.0;
    }

    pub fn set_cutoff_hz(&mut self, cutoff_hz: f32) {
        let safe = clampf(cutoff_hz, 10.0, (self.sample_rate * 0.45) as f32);
        self.a = 1.0 - (-TWO_PI * safe / self.sample_rate as f32).exp();
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }
}

pub struct Biquad {
    sample_rate: f64,
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    pub fn set_high_pass(&mut self, cutoff_hz: f32, q: f32) {
        let c = clampf(cutoff_hz, 10.0, (self.sample_rate * 0.45) as f32);
        let safe_q = q.max(0.05);
        let w0 = TWO_PI * c / self.sample_rate as f32;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * safe_q);
        self.set_coefficients(
            (1.0 + cos_w0) * 0.5,
            -(1.0 + cos_w0),
            (1.0 + cos_w0) * 0.5,
            1.0 + alpha,
            -2.0 * cos_w0,
            1.0 - alpha,
        );
    }

    pub fn set_band_pass(&mut self, center_hz: f32, q: f32) {
        let c = clampf(center_hz, 10.0, (self.sample_rate * 0.45) as f32);
        let safe_q = q.max(0.05);
        let w0 = TWO_PI * c / self.sample_rate as f32;
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * safe_q);
        self.set_coefficients(
            0.5 * sin_w0,
            0.0,
            -0.5 * sin_w0,
            1.0 + alpha,
            -2.0 * w0.cos(),
            1.0 - alpha,
        );
    }

    pub fn set_low_pass(&mut self, cutoff_hz: f32, q: f32) {
        let c = clampf(cutoff_hz, 10.0, (self.sample_rate * 0.45) as f32);
        let safe_q = q.max(0.05);
        let w0 = TWO_PI * c / self.sample_rate as f32;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * safe_q);
        self.set_coefficients(
            (1.0 - cos_w0) * 0.5,
            1.0 - cos_w0,
            (1.0 - cos_w0) * 0.5,
            1.0 + alpha,
            -2.0 * cos_w0,
            1.0 - alpha,
        );
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    fn set_coefficients(&mut self, b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) {
        let inv_a0 = 1.0 / a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }
}

#[inline]
pub fn decay_coef(sample_rate: f64, seconds: f32) -> f32 {
    let safe = seconds.max(0.0005);
    (-1.0 / (sample_rate * safe as f64) as f32).exp()
}

#[inline]
pub fn one_pole_coef(sample_rate: f64, seconds: f32) -> f32 {
    let safe = seconds.max(0.00001);
    (-1.0 / (sample_rate * safe as f64) as f32).exp()
}

#[inline]
pub fn decay_coef_t60(sample_rate: f64, seconds: f32) -> f32 {
    let safe = seconds.max(0.0001);
    (-T60_TO_TAU as f64 / (sample_rate * safe as f64)).exp() as f32
}

pub struct DecayEnvelope {
    sample_rate: f64,
    coef: f32,
    value: f32,
}

impl DecayEnvelope {
    pub fn new(sample_rate: f64) -> Self {
        let mut e = Self {
            sample_rate,
            coef: 0.999,
            value: 0.0,
        };
        e.set_decay_seconds(0.1);
        e
    }

    pub fn set_decay_seconds(&mut self, seconds: f32) {
        self.coef = decay_coef(self.sample_rate, seconds);
    }

    pub fn trigger(&mut self, level: f32) {
        self.value = self.value.max(level);
    }

    /// Part of the ported API (used by the C++ voices on stop); kept for
    /// fidelity even though our wrapper drives activity differently.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.value = 0.0;
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        self.value *= self.coef;
        if self.value < 1.0e-6 {
            self.value = 0.0;
        }
        self.value
    }

    pub fn is_active(&self) -> bool {
        self.value > 1.0e-6
    }
}

pub struct SweepSine {
    sample_rate: f64,
    phase: f32,
    current_hz: f32,
    target_hz: f32,
    amp: f32,
    amp_coef: f32,
    pitch_coef: f32,
    /// [196e] Bipolar decay curve shaping: 1.0 = fitted exponential,
    /// > 1 concave (punchier), < 1 convex (longer-feeling).
    curve: f32,
}

impl SweepSine {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            phase: 0.0,
            current_hz: 100.0,
            target_hz: 100.0,
            amp: 0.0,
            amp_coef: 0.999,
            pitch_coef: 0.001,
            curve: 1.0,
        }
    }

    /// Set the decay curve exponent (1.0 = fitted). Not reset by `trigger`.
    pub fn set_curve(&mut self, curve: f32) {
        self.curve = curve.max(0.05);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn trigger(
        &mut self,
        amplitude: f32,
        start_hz: f32,
        end_hz: f32,
        amp_decay: f32,
        pitch_decay: f32,
        phase_offset: f32,
    ) {
        self.amp = amplitude;
        self.phase = phase_offset;
        self.current_hz = start_hz;
        self.target_hz = end_hz;
        self.amp_coef = decay_coef(self.sample_rate, amp_decay);
        let safe_pitch = pitch_decay.max(0.0005);
        self.pitch_coef = 1.0 - (-1.0 / (self.sample_rate * safe_pitch as f64) as f32).exp();
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        self.current_hz += (self.target_hz - self.current_hz) * self.pitch_coef;
        self.phase += TWO_PI * self.current_hz / self.sample_rate as f32;
        if self.phase >= TWO_PI {
            self.phase -= TWO_PI;
        }
        // The == 1.0 shortcut keeps the fitted path bit-exact (golden [196]).
        let shaped_amp = if self.curve == 1.0 {
            self.amp
        } else {
            self.amp.powf(self.curve)
        };
        let out = self.phase.sin() * shaped_amp;
        self.amp *= self.amp_coef;
        if self.amp < 1.0e-6 {
            self.amp = 0.0;
        }
        out
    }

    /// Part of the ported API; kept for fidelity (see `DecayEnvelope::clear`).
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.phase = 0.0;
        self.current_hz = 100.0;
        self.target_hz = 100.0;
        self.amp = 0.0;
        self.amp_coef = 0.999;
        self.pitch_coef = 0.001;
    }

    pub fn is_active(&self) -> bool {
        self.amp > 1.0e-6
    }
}

/// White noise with a built-in DC blocker (as tuned in the original).
pub struct WhiteNoise {
    rng: Random,
    last_in: f32,
    last_out: f32,
}

impl WhiteNoise {
    pub fn new(seed: u32) -> Self {
        Self {
            rng: Random::new(seed),
            last_in: 0.0,
            last_out: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        let x = self.rng.bipolar();
        let y = x - self.last_in + 0.995 * self.last_out;
        self.last_in = x;
        self.last_out = y;
        y
    }
}
