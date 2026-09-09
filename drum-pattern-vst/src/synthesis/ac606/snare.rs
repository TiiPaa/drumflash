//! Super 606 Snare — audible match for the hardware sample.
//!
//! Ported from `Snare.hpp` of analogcode's "606-Inspired-Synth-Drums"
//! — MIT License, Copyright (c) 2026 Matthew Fecher (see `LICENSE-MIT.txt`).
//!
//! The tuned shell and the filtered wires are kept separate so Tune and
//! Snappy can move without pulling the other half around. The odd-looking
//! decimals below are the values fitted from the recordings.

use super::common::{clampf, flush_denormal, Biquad, OnePoleLpf, Random, T60_TO_TAU, TWO_PI};

const SNARE_REFERENCE_SAMPLE_RATE: f32 = 44100.0;
const SNARE_BODY_DURATION_SECONDS: f32 = 11016.0 / SNARE_REFERENCE_SAMPLE_RATE;
const SNARE_NOISE_DURATION_SECONDS: f32 = 16323.0 / SNARE_REFERENCE_SAMPLE_RATE;

const SNARE_BODY_SETTLED_HZ: f32 = 201.09442;
const SNARE_BODY_BEND_HZ: f32 = 149.23143;
const SNARE_BODY_BEND_TIME_SECONDS: f32 = 0.0133309;
const SNARE_BODY_ATTACK_TIME_SECONDS: f32 = 0.0007633316;
const SNARE_BODY_ATTACK_SHAPE: f32 = 3.2748916;
const SNARE_BODY_T60_SECONDS: f32 = 0.243;
const SNARE_BODY_LEVEL: f32 = 0.710;
const SNARE_BODY_TONE_LEVEL: f32 = 0.391;
const SNARE_BODY_TRANSIENT_LEVEL: f32 = 0.44;
const SNARE_BODY_TRANSIENT_T60_SECONDS: f32 = 0.030;
const SNARE_BODY_START_PHASE: f32 = 1.1711004;

const SNARE_IMPACT_START_SECONDS: f32 = 0.0002763113;
const SNARE_IMPACT_END_SECONDS: f32 = 0.0013649489;
const SNARE_IMPACT_LEVEL: f32 = -0.3000014;
const SNARE_IMPACT_DECAY_SECONDS: f32 = 0.0025407782;

const SNARE_WIRE_BAND1_HZ: f32 = 2998.628;
const SNARE_WIRE_BAND1_Q: f32 = 1.2771896;
const SNARE_WIRE_BAND2_HZ: f32 = 4596.2905;
const SNARE_WIRE_BAND2_Q: f32 = 0.30369505;
const SNARE_WIRE_BAND2_AMOUNT: f32 = 1.597958;
const SNARE_WIRE_LOW_PASS_HZ: f32 = 19677.396;
const SNARE_WIRE_ATTACK_SECONDS: f32 = 0.0031;
const SNARE_WIRE_ATTACK_SHAPE: f32 = 1.3;
const SNARE_WIRE_T60_SECONDS: f32 = 0.395;
const SNARE_WIRE_LEVEL: f32 = 1.28;
const SNARE_WIRE_RING_LEVEL: f32 = 0.014;
const SNARE_WIRE_RING_ATTACK_SECONDS: f32 = 0.012;
const SNARE_WIRE_RING_START_PHASE: f32 = -1.1;

const SNARE_FULL_DECAY_THRESHOLD: f32 = 0.999;
const SNARE_GATE_CURVE: f32 = 11.05;
const SNARE_GATE_TARGET_RATIO: f32 = 1.5881701e-5;

/// Exploration knobs ([196]): multipliers on the fitted constants.
/// `Default` reproduces the fitted sound.
#[derive(Clone, Copy, Debug)]
pub struct SnareMods {
    /// Shell pitch bend depth (fitted 1.0).
    pub shell_bend: f32,
    /// Level of the negative impact dip at the very start (fitted 1.0).
    pub impact: f32,
    /// Level of the low ring tying the wires to the shell (fitted 1.0).
    pub ring: f32,
}

impl Default for SnareMods {
    fn default() -> Self {
        Self {
            shell_bend: 1.0,
            impact: 1.0,
            ring: 1.0,
        }
    }
}

pub struct AcSnare {
    sample_rate: f32,
    decay: f32,
    pitch_ratio: f32,
    snappy_amount: f32,
    noise_color_ratio: f32,
    selected_duration_seconds: f32,
    gate_hold_seconds: f32,
    gate_fade_seconds: f32,
    body_phase: f32,
    noise_ring_phase: f32,
    noise_sample_rate_gain: f32,
    shell_bend: f32,
    impact: f32,
    ring: f32,
    frame_index: u64,
    natural_frame_count: u64,
    active: bool,
    random: Random,
    noise_band1: Biquad,
    noise_band2: Biquad,
    noise_low_pass: OnePoleLpf,
}

impl AcSnare {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        let sr = if sample_rate.is_finite() && sample_rate >= 8000.0 {
            sample_rate
        } else {
            SNARE_REFERENCE_SAMPLE_RATE
        };
        Self {
            sample_rate: sr,
            decay: 1.0,
            pitch_ratio: 1.0,
            snappy_amount: 1.0,
            noise_color_ratio: 1.0,
            selected_duration_seconds: SNARE_NOISE_DURATION_SECONDS,
            gate_hold_seconds: SNARE_NOISE_DURATION_SECONDS,
            gate_fade_seconds: 0.0,
            body_phase: 0.0,
            noise_ring_phase: 0.0,
            noise_sample_rate_gain: 1.0,
            shell_bend: 1.0,
            impact: 1.0,
            ring: 1.0,
            frame_index: 0,
            natural_frame_count: 0,
            active: false,
            random: Random::new(if seed == 0 { 0x606606 } else { seed }),
            noise_band1: Biquad::new(sr as f64),
            noise_band2: Biquad::new(sr as f64),
            noise_low_pass: OnePoleLpf::new(sr as f64),
        }
    }

    /// `decay_percent` 0..1, `pitch_ratio` (1 = original tuning),
    /// `snappy_amount` 0..1, `noise_color_ratio` (1 = original wire color).
    pub fn trigger(
        &mut self,
        decay_percent: f32,
        pitch_ratio: f32,
        snappy_amount: f32,
        noise_color_ratio: f32,
        mods: &SnareMods,
    ) {
        if !decay_percent.is_finite()
            || !pitch_ratio.is_finite()
            || !snappy_amount.is_finite()
            || !noise_color_ratio.is_finite()
            || pitch_ratio <= 0.0
            || noise_color_ratio <= 0.0
        {
            self.reset();
            return;
        }

        self.decay = clampf(decay_percent, 0.01, 1.0);
        self.pitch_ratio = clampf(pitch_ratio, 0.25, 4.0);
        self.snappy_amount = clampf(snappy_amount, 0.0, 1.0);
        self.noise_color_ratio = clampf(noise_color_ratio, 0.25, 4.0);
        self.shell_bend = mods.shell_bend.max(0.0);
        self.impact = mods.impact.max(0.0);
        self.ring = mods.ring.max(0.0);

        self.selected_duration_seconds = if self.decay >= SNARE_FULL_DECAY_THRESHOLD {
            SNARE_NOISE_DURATION_SECONDS
        } else {
            SNARE_NOISE_DURATION_SECONDS * self.decay
        };

        // Short settings fold the end down instead of chopping it off.
        let removed_duration = (SNARE_NOISE_DURATION_SECONDS - self.selected_duration_seconds).max(0.0);
        self.gate_fade_seconds = self.selected_duration_seconds.min(removed_duration);
        self.gate_hold_seconds = (self.selected_duration_seconds - self.gate_fade_seconds).max(0.0);

        self.natural_frame_count =
            (self.selected_duration_seconds * self.sample_rate).ceil().max(1.0) as u64;
        self.frame_index = 0;
        self.body_phase = SNARE_BODY_START_PHASE;
        self.noise_ring_phase = SNARE_WIRE_RING_START_PHASE;
        self.configure_noise_filters();
        self.active = true;
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        if !self.active || self.frame_index >= self.natural_frame_count {
            self.active = false;
            return 0.0;
        }

        let time = self.frame_index as f32 / self.sample_rate;
        let body = self.process_body(time);

        // Keep the wire stream moving at zero Snappy so later hits stay repeatable.
        let noise = self.process_noise(time) * self.snappy_amount;
        let output = flush_denormal(body + noise);

        self.frame_index += 1;
        if self.frame_index >= self.natural_frame_count {
            self.active = false;
        }
        if !output.is_finite() {
            self.reset();
            return 0.0;
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
        self.body_phase = 0.0;
        self.noise_ring_phase = 0.0;
        self.noise_band1.reset();
        self.noise_band2.reset();
        self.noise_low_pass.reset();
    }

    fn configure_noise_filters(&mut self) {
        self.noise_band1
            .set_band_pass(SNARE_WIRE_BAND1_HZ * self.noise_color_ratio, SNARE_WIRE_BAND1_Q);
        self.noise_band2
            .set_band_pass(SNARE_WIRE_BAND2_HZ * self.noise_color_ratio, SNARE_WIRE_BAND2_Q);
        self.noise_band1.reset();
        self.noise_band2.reset();
        self.noise_low_pass
            .set_cutoff_hz(SNARE_WIRE_LOW_PASS_HZ * self.noise_color_ratio);
        self.noise_low_pass.reset();
        self.noise_sample_rate_gain = (self.sample_rate / SNARE_REFERENCE_SAMPLE_RATE).sqrt();
    }

    fn decay_gate(&self, time: f32) -> f32 {
        if self.decay >= SNARE_FULL_DECAY_THRESHOLD || time < self.gate_hold_seconds {
            return 1.0;
        }
        if self.gate_fade_seconds <= 0.0 {
            return 0.0;
        }
        let progress = (time - self.gate_hold_seconds) / self.gate_fade_seconds;
        if progress >= 1.0 {
            return 0.0;
        }
        ((1.0 + SNARE_GATE_TARGET_RATIO) * (-SNARE_GATE_CURVE * progress).exp()
            - SNARE_GATE_TARGET_RATIO)
            .max(0.0)
    }

    fn process_body(&mut self, time: f32) -> f32 {
        let tuned_time = time * self.pitch_ratio;
        if tuned_time >= SNARE_BODY_DURATION_SECONDS || time >= self.selected_duration_seconds {
            return 0.0;
        }

        // I pitch the shell note but leave the short bend alone.
        // Moving both made the high notes dive too much.
        // [196] shell_bend scales the bend depth (fitted 1.0).
        let bend_hz = SNARE_BODY_BEND_HZ * self.shell_bend;
        let frequency = SNARE_BODY_SETTLED_HZ * self.pitch_ratio
            + bend_hz * (-time / SNARE_BODY_BEND_TIME_SECONDS).exp();
        let attack = (1.0 - (-time / SNARE_BODY_ATTACK_TIME_SECONDS).exp())
            .powf(SNARE_BODY_ATTACK_SHAPE);
        let envelope =
            attack * (-T60_TO_TAU * tuned_time / SNARE_BODY_T60_SECONDS).exp();
        let transient_envelope =
            attack * (-T60_TO_TAU * time / SNARE_BODY_TRANSIENT_T60_SECONDS).exp();

        let mut impact = 0.0;
        if time >= SNARE_IMPACT_START_SECONDS && time < SNARE_IMPACT_END_SECONDS {
            impact = SNARE_IMPACT_LEVEL
                * self.impact
                * (-(time - SNARE_IMPACT_START_SECONDS) / SNARE_IMPACT_DECAY_SECONDS).exp();
        }

        let output = ((SNARE_BODY_TONE_LEVEL * envelope
            + SNARE_BODY_TRANSIENT_LEVEL * transient_envelope)
            * self.body_phase.sin()
            + impact)
            * SNARE_BODY_LEVEL
            * self.decay_gate(time);
        self.body_phase = wrap_phase(
            self.body_phase + TWO_PI * frequency / self.sample_rate,
        );
        flush_denormal(output)
    }

    fn process_noise(&mut self, time: f32) -> f32 {
        if time >= self.selected_duration_seconds {
            return 0.0;
        }

        let white = self.random.bipolar_low24();
        let band1 = self.noise_band1.process(white);
        let band2 = self.noise_band2.process(white);
        let colored = self
            .noise_low_pass
            .process(band1 - SNARE_WIRE_BAND2_AMOUNT * band2);

        // Let the wires open over 3.1 ms instead of clicking on all at once.
        let attack_progress = (time / SNARE_WIRE_ATTACK_SECONDS).min(1.0);
        let attack = 1.0 - (1.0 - attack_progress).powf(SNARE_WIRE_ATTACK_SHAPE);
        let envelope = attack * (-T60_TO_TAU * time / SNARE_WIRE_T60_SECONDS).exp();
        let mut output = SNARE_WIRE_LEVEL * self.noise_sample_rate_gain * envelope * colored;

        // A quiet low ring ties the wire sound back to the shell.
        let ring_frequency = (SNARE_BODY_SETTLED_HZ
            + SNARE_BODY_BEND_HZ * self.shell_bend * (-time / SNARE_BODY_BEND_TIME_SECONDS).exp())
            * self.noise_color_ratio;
        let ring_attack = 1.0 - (-time / SNARE_WIRE_RING_ATTACK_SECONDS).exp();
        let ring_envelope = ring_attack * (-T60_TO_TAU * time / SNARE_WIRE_T60_SECONDS).exp();
        output += SNARE_WIRE_RING_LEVEL * self.ring * ring_envelope * self.noise_ring_phase.sin();
        self.noise_ring_phase = wrap_phase(
            self.noise_ring_phase + TWO_PI * ring_frequency / self.sample_rate,
        );
        flush_denormal(output * self.decay_gate(time))
    }
}

fn wrap_phase(mut phase: f32) -> f32 {
    while phase >= TWO_PI {
        phase -= TWO_PI;
    }
    while phase < 0.0 {
        phase += TWO_PI;
    }
    phase
}
