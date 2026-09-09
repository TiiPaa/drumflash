//! Super 606 Open & Closed Hi-Hats — one metallic source, two settings.
//!
//! Ported from `HiHats.hpp` of analogcode's "606-Inspired-Synth-Drums"
//! — MIT License, Copyright (c) 2026 Matthew Fecher (see `LICENSE-MIT.txt`).
//!
//! The metal is built one partial at a time: the author ran FFTs at a few
//! points in the open hat recording and kept the peaks that stayed put
//! through the decay. The 5978, 6340 and 7686 Hz lines carry most of the
//! audible ting; the rest was pulled down by ear so the middle does not turn
//! into a harsh chord.

use super::common::{
    clampf, decay_coef, flush_denormal, Biquad, Random, WhiteNoise, PI, TWO_PI,
};

#[derive(Clone, Copy)]
pub struct Partial {
    pub frequency_hz: f32,
    pub amplitude: f32,
    /// Bell lines get the short strike accent.
    pub bell: bool,
}

const fn p(frequency_hz: f32, amplitude: f32) -> Partial {
    Partial {
        frequency_hz,
        amplitude,
        bell: false,
    }
}

const fn bell(frequency_hz: f32, amplitude: f32) -> Partial {
    Partial {
        frequency_hz,
        amplitude,
        bell: true,
    }
}

static OPEN_HAT_PARTIALS: [Partial; 47] = [
    p(3804.0, 0.072),
    p(4270.0, 0.077),
    p(4500.0, 0.052),
    p(4632.0, 0.038),
    p(4800.0, 0.033),
    p(4900.0, 0.099),
    p(5140.0, 0.049),
    p(5400.0, 0.116),
    p(5500.0, 0.116),
    p(5610.0, 0.088),
    bell(5978.0, 0.742),
    bell(6340.0, 1.300),
    p(6500.0, 0.181),
    p(6600.0, 0.178),
    p(6701.0, 0.050),
    p(7106.0, 0.126),
    p(7200.0, 0.128),
    p(7339.0, 0.040),
    p(7500.0, 0.132),
    bell(7686.0, 0.573),
    p(7854.0, 0.086),
    p(7938.0, 0.039),
    p(8048.0, 0.062),
    p(8399.0, 0.078),
    p(8514.0, 0.081),
    p(8601.0, 0.050),
    p(8700.0, 0.045),
    p(8876.0, 0.166),
    p(9000.0, 0.051),
    p(9100.0, 0.081),
    p(9394.0, 0.101),
    p(9500.0, 0.052),
    p(9600.0, 0.064),
    p(9756.0, 0.045),
    p(9899.0, 0.042),
    p(10143.0, 0.062),
    p(10300.0, 0.034),
    p(10800.0, 0.043),
    p(11201.0, 0.037),
    p(11571.0, 0.036),
    p(11673.0, 0.041),
    p(12318.0, 0.060),
    p(12680.0, 0.090),
    p(12841.0, 0.033),
    p(13340.0, 0.045),
    p(13663.0, 0.034),
    p(16122.0, 0.030),
];

const MAX_PARTIAL_COUNT: usize = 64;

pub struct HiHatSpec {
    pub partials: &'static [Partial],
    pub noise_high_pass_hz: f32,
    pub noise_low_pass_hz: f32,
    /// Fitted tonal/noise balance and strike levels. These three are NOT
    /// read by the engine anymore: [196] routes them through `HatMods`
    /// (whose defaults reproduce these exact values). Kept as the reference
    /// documentation of the fitted sound.
    #[allow(dead_code)]
    pub tonal_mix: f32,
    pub noise_mix: f32,
    /// 0 skips this. Past about 0.6 the metal starts turning into noise.
    pub saturation_drive: f32,
    pub output_trim: f32,
    pub attack_time_constant_seconds: f32,
    /// Short filtered noise burst at the start so the hit gets through a mix.
    #[allow(dead_code)]
    pub click_amount: f32,
    pub click_decay_seconds: f32,
    /// Bell lines start louder, then settle after the hit.
    #[allow(dead_code)]
    pub bell_accent_amount: f32,
    pub bell_accent_decay_seconds: f32,
    /// Closed hat uses the fast decay. Open hat blends in the slow one.
    pub envelope_fast_weight: f32,
    pub fast_decay_seconds: f32,
    pub slow_decay_seconds: f32,
    /// True changes the ring itself. False only moves the outer gate.
    pub decay_scales_time_constants: bool,
    pub reference_duration_seconds: f32,
    pub minimum_decay_seconds: f32,
    pub minimum_duration_seconds: f32,
    /// Fade the gate so the open hat does not stop with a click.
    pub gate_fade_max_seconds: f32,
    /// A few Hz of slow drift keeps the long open hat from turning into a
    /// chord. Set the depth to 0 when the lines should stay put.
    pub line_wobble_depth: f32,
    pub line_wobble_correlation_seconds: f32,
}

/// One metal source, short envelope for closed and long envelope for open.
pub static CLOSED_HAT_SPEC: HiHatSpec = HiHatSpec {
    partials: &OPEN_HAT_PARTIALS,
    noise_high_pass_hz: 6800.0,
    noise_low_pass_hz: 12500.0,
    tonal_mix: 0.110,
    noise_mix: 0.909, // final balance set by ear
    saturation_drive: 0.60,
    output_trim: 1.75, // makeup after the saturation stage
    attack_time_constant_seconds: 0.0003, // gate edge, almost instant
    click_amount: 0.35,
    click_decay_seconds: 0.003,
    bell_accent_amount: 0.70,   // } stronger, snappier strike so
    bell_accent_decay_seconds: 0.040, // } the ting cuts in a full mix
    envelope_fast_weight: 1.0,  // single exponential
    fast_decay_seconds: 0.03355, // falls about 0.26 dB per ms
    slow_decay_seconds: 0.03355, // matches the fast curve here
    decay_scales_time_constants: true, // this moves the actual ring
                                    // instead of merely closing the gate later
    reference_duration_seconds: 9700.0 / 44100.0, // 44.1 kHz closed hat reference
    minimum_decay_seconds: 0.004,
    minimum_duration_seconds: 0.028,
    gate_fade_max_seconds: 0.045, // short fade at the gate
    line_wobble_depth: 0.0007,
    line_wobble_correlation_seconds: 0.020,
};

pub static OPEN_HAT_SPEC: HiHatSpec = HiHatSpec {
    partials: &OPEN_HAT_PARTIALS,
    noise_high_pass_hz: 6800.0,
    noise_low_pass_hz: 16000.0, // enough room for the shimmer
    tonal_mix: 0.110,
    noise_mix: 0.909, // final balance set by ear
    saturation_drive: 0.60,
    output_trim: 1.63, // makeup after the saturation stage
    attack_time_constant_seconds: 0.0003, // gate edge, almost instant
    click_amount: 0.35,
    click_decay_seconds: 0.003,
    bell_accent_amount: 0.70,   // } stronger, snappier strike so
    bell_accent_decay_seconds: 0.040, // } the ting cuts in a full mix
    envelope_fast_weight: 0.11, // a little snap over the wash
    fast_decay_seconds: 0.010,
    slow_decay_seconds: 0.580, // most of the tail lives here
    decay_scales_time_constants: true, // knob shortens the sizzle
                                    // while the outer gate cleans up the end
    reference_duration_seconds: 77864.0 / 44100.0, // 44.1 kHz open hat reference
    minimum_decay_seconds: 0.004,
    minimum_duration_seconds: 0.050,
    gate_fade_max_seconds: 0.500, // tucks the long tail away cleanly
    line_wobble_depth: 0.0007, // a few Hz keeps it breathing
    line_wobble_correlation_seconds: 0.020, // short enough that the lines
                                   // never line up into giant crest spikes
};

const FULL_LEVEL_SAMPLE_RATE_RATIO: f32 = 0.40;
const MUTED_SAMPLE_RATE_RATIO: f32 = 0.48;

/// Dedicated low-pass biquad with denormal flushing in the state update
/// (the shared `Biquad` flushes only via the caller in the original).
struct LowPassBiquad {
    sample_rate: f64,
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl LowPassBiquad {
    fn new(sample_rate: f64) -> Self {
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

    fn set_low_pass(&mut self, cutoff_hz: f32, q: f32) {
        let cutoff = clampf(cutoff_hz, 10.0, (self.sample_rate * 0.45) as f32);
        let safe_q = q.max(0.05);
        let w0 = TWO_PI * cutoff / self.sample_rate as f32;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * safe_q);
        let inverse_a0 = 1.0 / (1.0 + alpha);
        self.b0 = ((1.0 - cos_w0) * 0.5) * inverse_a0;
        self.b1 = (1.0 - cos_w0) * inverse_a0;
        self.b2 = self.b0;
        self.a1 = (-2.0 * cos_w0) * inverse_a0;
        self.a2 = (1.0 - alpha) * inverse_a0;
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    #[inline]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = flush_denormal(self.b1 * input - self.a1 * output + self.z2);
        self.z2 = flush_denormal(self.b2 * input - self.a2 * output);
        output
    }
}

/// Exploration knobs ([196]). `Default` reproduces the fitted spec sound.
#[derive(Clone, Copy, Debug)]
pub struct HatMods {
    /// Tonal (partials) mix — fitted 0.110. The noise mix stays at the spec's.
    pub metal: f32,
    /// Initial noise burst level — fitted 0.35.
    pub click: f32,
    /// Bell lines (5978/6340/7686 Hz) strike accent — fitted 0.70.
    pub bell: f32,
    /// Multiplier on the spec's line wobble depth (fitted 1.0).
    pub wobble: f32,
    /// Deterministic detune of the partials, 0 = fitted, 1 = ±1.5 %.
    pub spread: f32,
    /// Spectral tilt of the partial amplitudes (-1..1, 0 = fitted).
    pub brightness: f32,
}

impl Default for HatMods {
    fn default() -> Self {
        Self {
            metal: 0.110,
            click: 0.35,
            bell: 0.70,
            wobble: 1.0,
            spread: 0.0,
            brightness: 0.0,
        }
    }
}

pub struct AcMetalHat {
    sample_rate: f64,
    phase_random: Random,
    wobble_random: Random,
    noise: WhiteNoise,
    noise_high_pass: Biquad,
    noise_low_pass: LowPassBiquad,
    dc_blocker: super::common::DcBlocker,
    phases: [f32; MAX_PARTIAL_COUNT],
    increments: [f32; MAX_PARTIAL_COUNT],
    amplitudes: [f32; MAX_PARTIAL_COUNT],
    wobble_states: [f32; MAX_PARTIAL_COUNT],
    bell_flags: [bool; MAX_PARTIAL_COUNT],
    wobble_alpha: f32,
    wobble_drive: f32,
    bell_accent_amount: f32,
    bell_accent_coefficient: f32,
    bell_accent_envelope: f32,
    click_amount: f32,
    click_coefficient: f32,
    click_envelope: f32,
    active_partial_count: usize,
    tonal_mix: f32,
    noise_mix: f32,
    saturation_drive: f32,
    output_trim: f32,
    attack_coefficient: f32,
    attack_envelope: f32,
    fast_decay_coefficient: f32,
    slow_decay_coefficient: f32,
    fast_envelope_weight: f32,
    fast_envelope: f32,
    slow_envelope: f32,
    natural_frame_count: u64,
    gate_fade_frames: u64,
    inverse_gate_fade_frames: f32,
    frame_index: u64,
    active: bool,
}

impl AcMetalHat {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        let sr = if sample_rate.is_finite() && sample_rate >= 8000.0 {
            sample_rate as f64
        } else {
            44100.0
        };
        let safe_seed = if seed == 0 { 0x606606 } else { seed };
        Self {
            sample_rate: sr,
            phase_random: Random::new(safe_seed ^ 0x9E3779B9),
            wobble_random: Random::new(safe_seed ^ 0x51D3B7A1),
            noise: WhiteNoise::new(safe_seed ^ 0xA511E9B3),
            noise_high_pass: Biquad::new(sr),
            noise_low_pass: LowPassBiquad::new(sr),
            dc_blocker: super::common::DcBlocker::new(),
            phases: [0.0; MAX_PARTIAL_COUNT],
            increments: [0.0; MAX_PARTIAL_COUNT],
            amplitudes: [0.0; MAX_PARTIAL_COUNT],
            wobble_states: [0.0; MAX_PARTIAL_COUNT],
            bell_flags: [false; MAX_PARTIAL_COUNT],
            wobble_alpha: 0.0,
            wobble_drive: 0.0,
            bell_accent_amount: 0.0,
            bell_accent_coefficient: 0.999,
            bell_accent_envelope: 0.0,
            click_amount: 0.0,
            click_coefficient: 0.999,
            click_envelope: 0.0,
            active_partial_count: 0,
            tonal_mix: 0.0,
            noise_mix: 0.0,
            saturation_drive: 0.0,
            output_trim: 1.0,
            attack_coefficient: 0.01,
            attack_envelope: 0.0,
            fast_decay_coefficient: 0.999,
            slow_decay_coefficient: 0.999,
            fast_envelope_weight: 1.0,
            fast_envelope: 0.0,
            slow_envelope: 0.0,
            natural_frame_count: 0,
            gate_fade_frames: 0,
            inverse_gate_fade_frames: 0.0,
            frame_index: 0,
            active: false,
        }
    }

    /// `spec`: closed or open hat, `decay_percent` 0..1,
    /// `pitch_ratio` (1 = original tuning), `mods`: exploration knobs.
    pub fn trigger(
        &mut self,
        spec: &'static HiHatSpec,
        decay_percent: f32,
        pitch_ratio: f32,
        mods: &HatMods,
    ) {
        if !decay_percent.is_finite() || !pitch_ratio.is_finite() || pitch_ratio <= 0.0 {
            self.reset();
            return;
        }

        let decay = clampf(decay_percent, 0.0, 1.0);
        let frequency_ratio = clampf(pitch_ratio, 1.0 / 16.0, 16.0);
        let spread = clampf(mods.spread, 0.0, 1.0);
        let brightness = clampf(mods.brightness, -1.0, 1.0);

        self.active_partial_count = spec.partials.len().min(MAX_PARTIAL_COUNT);
        for index in 0..self.active_partial_count {
            // [196] Spread: deterministic per-partial detune (0 at spread 0).
            let det = ((index as u32).wrapping_mul(2654435761) >> 8) as f32 / 16777215.0 * 2.0
                - 1.0;
            let frequency_hz = spec.partials[index].frequency_hz
                * frequency_ratio
                * (1.0 + spread * 0.015 * det);
            let sample_rate_ratio = frequency_hz / self.sample_rate as f32;
            let mut level = 1.0f32;
            if sample_rate_ratio >= MUTED_SAMPLE_RATE_RATIO {
                level = 0.0;
            } else if sample_rate_ratio > FULL_LEVEL_SAMPLE_RATE_RATIO {
                let taper = (MUTED_SAMPLE_RATE_RATIO - sample_rate_ratio)
                    / (MUTED_SAMPLE_RATE_RATIO - FULL_LEVEL_SAMPLE_RATE_RATIO);
                level = taper * taper * (3.0 - 2.0 * taper);
            }

            // Start phases close enough to land as one hit.
            // Perfect alignment clips, fully random phases lose the attack.
            self.phases[index] = (self.phase_random.bipolar() + 1.0) * 0.3 * PI;
            self.increments[index] = if level > 0.0 {
                TWO_PI * sample_rate_ratio
            } else {
                0.0
            };
            // [196] Brightness: spectral tilt around the loudest bell line.
            let tilt = (spec.partials[index].frequency_hz / 6340.0).powf(brightness * 0.75);
            self.amplitudes[index] = spec.partials[index].amplitude * level * tilt;
            self.bell_flags[index] = spec.partials[index].bell;
        }
        self.bell_accent_amount = mods.bell.max(0.0);
        self.bell_accent_coefficient =
            decay_coef(self.sample_rate, spec.bell_accent_decay_seconds.max(0.005));
        self.bell_accent_envelope = 1.0;
        self.click_amount = mods.click.max(0.0);
        self.click_coefficient = decay_coef(self.sample_rate, spec.click_decay_seconds.max(0.0005));
        self.click_envelope = 1.0;

        self.noise_high_pass
            .set_high_pass(spec.noise_high_pass_hz * frequency_ratio, 0.70);
        self.noise_low_pass
            .set_low_pass(spec.noise_low_pass_hz * frequency_ratio, 0.707);
        self.noise_high_pass.reset();
        self.noise_low_pass.reset();
        self.dc_blocker.reset();

        self.tonal_mix = mods.metal.max(0.0);
        self.noise_mix = spec.noise_mix;
        self.saturation_drive = spec.saturation_drive.max(0.0);
        self.output_trim = spec.output_trim;

        let wobble_depth = spec.line_wobble_depth * mods.wobble.max(0.0);
        if wobble_depth > 0.0 {
            self.wobble_alpha = 1.0
                - decay_coef(
                    self.sample_rate,
                    spec.line_wobble_correlation_seconds.max(0.005),
                );
            // Scale the noise so correlation does not change the wobble depth.
            self.wobble_drive = wobble_depth * (6.0 * self.wobble_alpha).sqrt();
        } else {
            self.wobble_alpha = 0.0;
            self.wobble_drive = 0.0;
        }
        self.wobble_states = [0.0; MAX_PARTIAL_COUNT];

        let time_constant_scale = if spec.decay_scales_time_constants {
            decay
        } else {
            1.0
        };
        self.fast_decay_coefficient = decay_coef(
            self.sample_rate,
            spec.minimum_decay_seconds
                .max(time_constant_scale * spec.fast_decay_seconds),
        );
        self.slow_decay_coefficient = decay_coef(
            self.sample_rate,
            spec.minimum_decay_seconds
                .max(time_constant_scale * spec.slow_decay_seconds),
        );
        self.fast_envelope_weight = clampf(spec.envelope_fast_weight, 0.0, 1.0);
        self.fast_envelope = 1.0;
        self.slow_envelope = 1.0;
        self.attack_coefficient = 1.0 - decay_coef(self.sample_rate, spec.attack_time_constant_seconds);
        self.attack_envelope = 0.0;

        let natural_duration_seconds = spec
            .minimum_duration_seconds
            .max(decay * spec.reference_duration_seconds);
        self.natural_frame_count =
            (natural_duration_seconds as f64 * self.sample_rate + 0.5).floor().max(1.0) as u64;
        let mut gate_fade_frames =
            (spec.gate_fade_max_seconds as f64 * self.sample_rate + 0.5).floor().max(0.0) as u64;
        gate_fade_frames = gate_fade_frames.min(self.natural_frame_count);
        self.gate_fade_frames = gate_fade_frames;
        self.inverse_gate_fade_frames = if gate_fade_frames > 0 {
            1.0 / gate_fade_frames as f32
        } else {
            0.0
        };
        self.frame_index = 0;
        self.active = true;
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        if !self.active || self.frame_index >= self.natural_frame_count {
            self.active = false;
            return 0.0;
        }

        if self.wobble_drive > 0.0 {
            for index in 0..self.active_partial_count {
                self.wobble_states[index] += self.wobble_drive * self.wobble_random.bipolar()
                    - self.wobble_alpha * self.wobble_states[index];
            }
        }

        let bell_gain = 1.0 + self.bell_accent_amount * self.bell_accent_envelope;
        self.bell_accent_envelope =
            flush_denormal(self.bell_accent_envelope * self.bell_accent_coefficient);

        let mut tonal = 0.0f32;
        for index in 0..self.active_partial_count {
            let line_gain = if self.bell_flags[index] { bell_gain } else { 1.0 };
            tonal += self.phases[index].sin() * self.amplitudes[index] * line_gain;
            self.phases[index] += self.increments[index] * (1.0 + self.wobble_states[index]);
            if self.phases[index] >= TWO_PI {
                self.phases[index] -= TWO_PI;
            }
        }

        let noise = self
            .noise_low_pass
            .process(self.noise_high_pass.process(self.noise.process()));

        self.attack_envelope += self.attack_coefficient * (1.0 - self.attack_envelope);
        self.fast_envelope = flush_denormal(self.fast_envelope * self.fast_decay_coefficient);
        self.slow_envelope = flush_denormal(self.slow_envelope * self.slow_decay_coefficient);
        let mut envelope = self.attack_envelope
            * (self.fast_envelope_weight * self.fast_envelope
                + (1.0 - self.fast_envelope_weight) * self.slow_envelope);
        if self.gate_fade_frames > 0 {
            let frames_remaining = self.natural_frame_count - self.frame_index;
            if frames_remaining < self.gate_fade_frames {
                envelope *= frames_remaining as f32 * self.inverse_gate_fade_frames;
            }
        }

        let mut source = tonal * self.tonal_mix + noise * self.noise_mix;
        if self.saturation_drive > 0.0 {
            source = (source * self.saturation_drive).tanh();
        }
        let body = source * envelope + noise * self.click_amount * self.click_envelope;
        self.click_envelope = flush_denormal(self.click_envelope * self.click_coefficient);
        let mut output = body * self.output_trim;
        output = flush_denormal(self.dc_blocker.process(output));
        if !output.is_finite() {
            self.reset();
            return 0.0;
        }

        self.frame_index += 1;
        if self.frame_index >= self.natural_frame_count {
            self.active = false;
        }
        output
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.frame_index = 0;
        self.natural_frame_count = 0;
        self.gate_fade_frames = 0;
        self.inverse_gate_fade_frames = 0.0;
        self.attack_envelope = 0.0;
        self.fast_envelope = 0.0;
        self.slow_envelope = 0.0;
        self.noise_high_pass.reset();
        self.noise_low_pass.reset();
        self.dc_blocker.reset();
        self.active_partial_count = 0;
        self.wobble_alpha = 0.0;
        self.wobble_drive = 0.0;
        self.bell_accent_amount = 0.0;
        self.bell_accent_envelope = 0.0;
        self.click_amount = 0.0;
        self.click_envelope = 0.0;
        self.phases = [0.0; MAX_PARTIAL_COUNT];
        self.increments = [0.0; MAX_PARTIAL_COUNT];
        self.amplitudes = [0.0; MAX_PARTIAL_COUNT];
        self.wobble_states = [0.0; MAX_PARTIAL_COUNT];
        self.bell_flags = [false; MAX_PARTIAL_COUNT];
    }
}
