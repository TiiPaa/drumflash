//! Super 606 XL BassDrum ("608").
//!
//! Ported from `BassDrum.hpp` of analogcode's "606-Inspired-Synth-Drums"
//! — MIT License, Copyright (c) 2026 Matthew Fecher (see `LICENSE-MIT.txt`).
//!
//! A swept-sine body with filtered noise and impulse transients. The impulse
//! is derived from the body's own slope (`body_delta`), which keeps the
//! transient glued to the note.

use super::common::{
    clampf, lerpf, soft, Biquad, DcBlocker, DecayEnvelope, OnePoleLpf, SweepSine, WhiteNoise, PI,
};

const VOICE_OUTPUT_TRIM: f32 = 0.70;
const XL_DECAY_MIN: f32 = -1.75;
const XL_DECAY_MAX: f32 = 3.00;

/// Exploration knobs for the 608 engine ([196]). `Default` reproduces the
/// fitted sound; nothing here is meant to stay hardware-faithful.
#[derive(Clone, Copy, Debug)]
pub struct BdMods {
    /// Noise "tick" level (0..1, fitted 0.2).
    pub click: f32,
    /// Click lowpass cutoff in Hz (fitted 1400).
    pub click_tone_hz: f32,
    /// Body-slope impulse level (0..2, fitted 0.2).
    pub punch: f32,
    /// Body lowpass tone (0..2, fitted 0.34; past 1.0 the filter opens to 6 kHz).
    pub tone: f32,
    /// Internal tanh drive (0..2, fitted 0.18; past it a quadratic boost kicks in).
    pub drive: f32,
    /// Body sweep range multiplier (0..2 → x0..x4, fitted 0.5 = x1).
    pub sweep: f32,
    /// Pitch bend speed multiplier (0..1 → x0..x2, fitted 0.5 = x1).
    pub bend: f32,
    /// Bipolar decay curve (-1..1, fitted 0 = exponential): γ = 2^(3c),
    /// so +1 = x8 concave (very punchy), -1 = x0.125 convex (long tail feel).
    pub decay_curve: f32,
}

impl Default for BdMods {
    fn default() -> Self {
        Self {
            click: 0.2,
            click_tone_hz: 1400.0,
            punch: 0.2,
            tone: 0.34,
            drive: 0.18,
            sweep: 0.5,
            bend: 0.5,
            decay_curve: 0.0,
        }
    }
}

struct BassDrum {
    body: SweepSine,
    click_env: DecayEnvelope,
    impulse_env: DecayEnvelope,
    click_lpf: OnePoleLpf,
    body_lpf: OnePoleLpf,
    impulse_lpf: OnePoleLpf,
    impulse_hpf: Biquad,
    dc: DcBlocker,
    last_body_raw: f32,

    level: f32,
    tune: f32,
    tone: f32,
    decay: f32,
    attack: f32,
    drive: f32,
    tune_semitones: f32,
    click_amount: f32,
    impulse_amount: f32,
    impulse_decay: f32,
    sweep_scale: f32,
    bend_scale: f32,
}

impl BassDrum {
    fn new(sample_rate: f64) -> Self {
        let mut click_lpf = OnePoleLpf::new(sample_rate);
        click_lpf.set_cutoff_hz(1400.0);
        let mut body_lpf = OnePoleLpf::new(sample_rate);
        body_lpf.set_cutoff_hz(900.0);
        let mut impulse_hpf = Biquad::new(sample_rate);
        impulse_hpf.set_high_pass(1350.0, 0.707);
        let mut impulse_lpf = OnePoleLpf::new(sample_rate);
        impulse_lpf.set_cutoff_hz(2600.0);
        Self {
            body: SweepSine::new(sample_rate),
            click_env: DecayEnvelope::new(sample_rate),
            impulse_env: DecayEnvelope::new(sample_rate),
            click_lpf,
            body_lpf,
            impulse_lpf,
            impulse_hpf,
            dc: DcBlocker::new(),
            last_body_raw: 0.0,
            level: 1.0,
            tune: 0.28,
            tone: 0.34,
            decay: 0.72,
            attack: 0.25,
            drive: 0.18,
            tune_semitones: 2.0,
            click_amount: 0.25,
            impulse_amount: 0.25,
            impulse_decay: 0.2,
            sweep_scale: 1.0,
            bend_scale: 1.0,
        }
    }

    fn apply_defaults(&mut self) {
        self.level = 1.0;
        self.tune = 0.28;
        self.tone = 0.34;
        self.decay = 0.72;
        self.attack = 0.25;
        self.drive = 0.18;
        self.tune_semitones = 2.0;
        self.click_amount = 0.25;
        self.impulse_amount = 0.25;
        self.impulse_decay = 0.2;
        self.sweep_scale = 1.0;
        self.bend_scale = 1.0;
    }

    fn trigger(&mut self, accent: f32) {
        let a = clampf(accent, 0.0, 1.0);
        let t = self.tune - 0.5;
        let ratio = 2.0f32.powf(self.tune_semitones / 12.0);
        let base_start_hz = (120.0 + t * 16.0) * ratio;
        let end_hz = (53.0 + t * 8.0) * ratio;
        // [196] Sweep: scale the startHz→endHz span around the landing note.
        // (kept bit-exact at the fitted x1 so the golden render is stable)
        let start_hz = if self.sweep_scale == 1.0 {
            base_start_hz
        } else {
            end_hz + (base_start_hz - end_hz) * self.sweep_scale
        };
        let transient_shape =
            clampf((self.attack + self.click_amount + self.impulse_amount) / 3.0, 0.0, 1.5);
        let thud_shape = clampf(transient_shape * transient_shape, 0.0, 1.5);
        let body_start_hz = start_hz * (1.0 + thud_shape * 0.28);
        let body_amp = 0.92 + thud_shape * 0.22 + a * 0.08;
        let body_pitch_decay =
            lerpf(0.022, 0.008, clampf(thud_shape, 0.0, 1.0)) * self.bend_scale;
        self.body.trigger(
            body_amp,
            body_start_hz,
            end_hz,
            0.22 + self.decay * 0.12,
            body_pitch_decay,
            -0.33 * PI,
        );
        self.click_env
            .set_decay_seconds(0.0020 + self.attack * 0.0045 + self.click_amount * 0.0040);
        self.click_env.trigger(
            0.02 + self.attack * 0.06 + self.click_amount * 0.28 + a * 0.02,
        );
        self.impulse_env.set_decay_seconds(
            // [196] Past the fitted 1.0 the decay keeps opening (up to ~39 ms
            // at 2.0) so Punch Decay is clearly audible.
            0.0008 + self.impulse_decay.min(1.0) * 0.0080
                + (self.impulse_decay - 1.0).max(0.0) * 0.03,
        );
        self.impulse_env.trigger(
            0.02 + self.impulse_amount * 0.55 + thud_shape * 0.08 + a * 0.04,
        );
        // [196] Past the fitted 1.0 the body filter keeps opening toward
        // 6 kHz — 620..1200 Hz below, so the fitted tone is unchanged.
        // [196d] Below the fitted 0.34 it now sweeps down to 80 Hz: the body
        // is a near-pure sine at 53-120 Hz, so cutoffs above ~600 Hz barely
        // do anything — the audible range is UNDER the fitted band.
        // The fitted point (0.34 → 817 Hz) is preserved bit-exactly.
        let body_cutoff = if self.tone == 0.34 {
            lerpf(620.0, 1200.0, 0.34)
        } else if self.tone < 0.34 {
            lerpf(80.0, lerpf(620.0, 1200.0, 0.34), self.tone / 0.34)
        } else if self.tone <= 1.0 {
            lerpf(lerpf(620.0, 1200.0, 0.34), 1200.0, (self.tone - 0.34) / 0.66)
        } else {
            lerpf(1200.0, 6000.0, self.tone - 1.0)
        };
        self.body_lpf.set_cutoff_hz(body_cutoff);
        self.last_body_raw = 0.0;
    }

    fn process(&mut self, noise: f32) -> f32 {
        let body_raw = self.body.process();
        let body = self.body_lpf.process(body_raw);
        let click = self.click_lpf.process(noise) * self.click_env.process();
        let body_delta = body_raw - self.last_body_raw;
        self.last_body_raw = body_raw;
        let impulse_core = self.impulse_lpf.process(self.impulse_hpf.process(body_delta));
        let impulse = impulse_core * self.impulse_env.process();
        let out = body * (0.92 + self.drive * 0.10)
            + click * (0.06 + self.click_amount * 0.90)
            + impulse * (0.08 + self.impulse_amount * 2.20);

        // [196] Past the fitted 0.18 the drive bends sharply upward (up to
        // ~8x at 2.0) — below it the mapping is the fitted one.
        let base_drive = 1.12 + self.drive * 0.12 + (self.drive - 0.18).max(0.0).powi(2) * 2.0;
        let shaped = soft(out * base_drive);
        self.dc.process(shaped * self.level)
    }

    fn is_active(&self) -> bool {
        self.body.is_active() || self.click_env.is_active() || self.impulse_env.is_active()
    }
}

/// Wrapper voice: owns the noise stream and the silence gate.
pub struct AcBassDrum {
    noise: WhiteNoise,
    drum: BassDrum,
    active: bool,
    silence_frames: i32,
}

impl AcBassDrum {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        Self {
            noise: WhiteNoise::new(seed),
            drum: BassDrum::new(sample_rate as f64),
            active: false,
            silence_frames: 0,
        }
    }

    /// `decay_percent` 0..1, `tune_semitones`,
    /// `analog_pitch_jitter_semitones`, `mods`: exploration knobs (defaults =
    /// fitted sound).
    pub fn trigger(
        &mut self,
        decay_percent: f32,
        tune_semitones: f32,
        analog_pitch_jitter_semitones: f32,
        mods: &BdMods,
    ) {
        self.drum.apply_defaults();
        let decay = clampf(decay_percent, 0.0, 1.0);
        // Square the decay knob so most of the XL kick stays short and punchy.
        // The last part opens into the long tail.
        let shaped_decay = decay * decay;
        self.drum.decay = lerpf(XL_DECAY_MIN, XL_DECAY_MAX, shaped_decay);
        self.drum.attack = 0.2; // fitted effective value (Attack slider removed [196b])
        self.drum.tune_semitones = tune_semitones + analog_pitch_jitter_semitones;
        self.drum.click_amount = clampf(mods.click, 0.0, 1.0);
        self.click_lpf_set(mods.click_tone_hz);
        self.drum.impulse_amount = clampf(mods.punch, 0.0, 2.0);
        self.drum.impulse_decay = 0.2; // fitted (Punch Decay slider removed [196b])
        self.drum.tone = clampf(mods.tone, 0.0, 2.0);
        self.drum.drive = clampf(mods.drive, 0.0, 2.0);
        self.drum.sweep_scale = clampf(mods.sweep, 0.0, 2.0) * 2.0;
        self.drum.bend_scale = clampf(mods.bend, 0.0, 1.0) * 2.0;
        // [196e] Bipolar decay curve → exponent on the body amp envelope.
        // γ = 2^(3c): +1 = x8 concave (very punchy), -1 = x0.125 convex
        // (very long tail feel). 0 = fitted exponential.
        self.drum
            .body
            .set_curve(2.0f32.powf(3.0 * clampf(mods.decay_curve, -1.0, 1.0)));
        self.drum.trigger(1.0);
        self.active = true;
        self.silence_frames = 0;
    }

    fn click_lpf_set(&mut self, hz: f32) {
        self.drum.click_lpf.set_cutoff_hz(clampf(hz, 400.0, 4000.0));
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let out = self.drum.process(self.noise.process()) * VOICE_OUTPUT_TRIM;
        if self.drum.is_active() {
            self.silence_frames = 0;
        } else if out.abs() < 1.0e-5 {
            self.silence_frames += 1;
            if self.silence_frames >= 32 {
                self.active = false;
            }
        } else {
            self.silence_frames = 0;
        }
        out
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.silence_frames = 0;
    }
}
