//! TR-606 snare drum — separate voice modelling the bridged-T resonator
//! topology of the original Roland TR-606.
//!
//! Signal chain (matches the user's spec):
//! ```text
//!   White noise ──► passive LP (~3 kHz "softener") ──┬──► × env (Swing VCA) ──► Biquad bandpass (bridged-T) ──┐
//!                                                    │                                                          ├──► mix → out
//!                                                    └────────────────────► × env ──────────────────────────────┘
//!                                                                          (dry wires / rattle layer)
//! ```
//!
//! All filter / resonator parameters are exposed:
//! - `settings.frequency`       → bandpass centre (~150–400 Hz typical)
//! - `settings.filter_freq`     → LP softener cutoff (Hz)
//! - `settings.special[0]`      → resonance Q (0.5 .. 12)
//! - `settings.special[1]`      → tone mix: 0 = mostly wires, 1 = mostly body
//! - `settings.special[2]`      → wire crispness: HP gain of the dry layer

use super::{dsp, saturation, settings::snare606::Snare606Settings, Voice, VoiceSettings};

/// Anti-click floor for the amplitude attack (a true 0 ms attack is a step = click).
const MIN_AMP_ATTACK_MS: f32 = 0.2;

pub struct Snare606Voice {
    settings: Snare606Settings,
    sample_rate: f32,

    noise: dsp::WhiteNoise,
    noise_r: dsp::WhiteNoise,
    lp_softener: dsp::OnePoleFilter,
    lp_softener_r: dsp::OnePoleFilter,
    /// Highpass on the dry wires layer — keeps it crisp on top of the body.
    wires_hp: dsp::OnePoleFilter,
    wires_hp_r: dsp::OnePoleFilter,
    resonator: dsp::Biquad,
    resonator_r: dsp::Biquad,
    envelope: dsp::DecayReleaseEnvelope,
    /// Filter envelope that closes the LP softener after the trigger.
    filter_env: dsp::ExpDecayEnvelope,
    /// Saturation stage for analog character.
    saturation: saturation::SaturationConfig,
    // DC blockers (per channel).
    dc_block_l: dsp::DcBlocker,
    dc_block_r: dsp::DcBlocker,
    /// Per-hit analog drift (breathing) — pitch/level/time variation per hit.
    drift: dsp::AnalogDrift,

    active: bool,
}

impl Snare606Voice {
    pub fn new(sample_rate: f32, settings: Snare606Settings) -> Self {
        let mut lp_softener = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        lp_softener.set_cutoff(settings.filter_freq.max(500.0), sample_rate);
        let mut lp_softener_r = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        lp_softener_r.set_cutoff(settings.filter_freq.max(500.0), sample_rate);

        let mut wires_hp = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        wires_hp.set_cutoff(1500.0, sample_rate);
        let mut wires_hp_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        wires_hp_r.set_cutoff(1500.0, sample_rate);

        let mut resonator = dsp::Biquad::new();
        let q = settings.resonance.clamp(0.5, 12.0);
        resonator.set_bandpass(settings.frequency.max(80.0), q, sample_rate);
        let mut resonator_r = dsp::Biquad::new();
        resonator_r.set_bandpass(settings.frequency.max(80.0), q, sample_rate);

        let mut envelope = dsp::DecayReleaseEnvelope::new(
            sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms((settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
        envelope.set_hold(settings.hold);

        let filter_env =
            dsp::ExpDecayEnvelope::new(sample_rate, 8.0, settings.filter_env_decay.max(0.001))
                .with_attack_ms(0.3);

        // Saturation stage — initialized with defaults (disabled)
        let saturation = saturation::SaturationConfig {
            saturation_type: saturation::SaturationType::None,
            amount: 0.0,
            mix: 1.0,
            output_gain: 1.0,
            pre_filter: false,
        };

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(0x5A5A_5A5A),
            noise_r: dsp::WhiteNoise::new(0xA5A5_A5A5),
            lp_softener,
            lp_softener_r,
            wires_hp,
            wires_hp_r,
            resonator,
            resonator_r,
            envelope,
            filter_env,
            saturation,
            dc_block_l: dsp::DcBlocker::default(),
            dc_block_r: dsp::DcBlocker::default(),
            drift: dsp::AnalogDrift::new(0x6060_6060),
            active: false,
        }
    }

    fn resonance_q(&self) -> f32 {
        self.settings.resonance.clamp(0.5, 12.0)
    }

    fn tone_mix(&self) -> f32 {
        self.settings.tone.clamp(0.0, 1.0)
    }

    fn wire_crisp(&self) -> f32 {
        self.settings.snap.clamp(0.0, 1.0)
    }
}

impl Voice for Snare606Voice {
    fn trigger(&mut self) {
        let was_active = self.active;
        self.active = true;
        // Cold start only (voice was silent): reset the filters + resonator for a
        // clean, deterministic attack. NEVER on a ringing-tail retrigger — resetting
        // a ringing resonator/filter is a discontinuity (click). Noise is continuous.
        if !was_active {
            self.lp_softener.reset();
            self.lp_softener_r.reset();
            self.wires_hp.reset();
            self.wires_hp_r.reset();
            self.resonator.reset();
            self.resonator_r.reset();
            self.dc_block_l.reset();
            self.dc_block_r.reset();
        }
        // analog = per-hit drift (breathing) ; digital = bit-identical hits.
        self.drift.trigger(self.settings.analog >= 0.5);
        self.envelope
            .set_decay(self.settings.decay * self.drift.time);
        self.envelope
            .set_release(self.settings.release * self.drift.time);
        // Drift the resonator pitch so the tonal body varies per hit.
        let drifted_freq = self.settings.frequency * self.drift.pitch;
        let q = self.resonance_q();
        self.resonator
            .set_bandpass(drifted_freq.max(80.0), q, self.sample_rate);
        self.resonator_r
            .set_bandpass(drifted_freq.max(80.0), q, self.sample_rate);
        self.envelope.trigger();
        self.filter_env.trigger();
    }

    fn trigger_hard(&mut self) {
        self.active = true;
        self.envelope.trigger_hard();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let env = self.envelope.next();
        if !self.envelope.is_active() {
            self.active = false;
            return 0.0;
        }

        // Filter envelope closes the LP softener (×4 depth).
        let filter_env_val = self.filter_env.next();
        let modulated_cutoff = self.settings.filter_freq
            * (1.0 + filter_env_val * self.settings.filter_env_amount * 4.0);
        let cutoff = modulated_cutoff.max(100.0);
        self.lp_softener.set_cutoff(cutoff, self.sample_rate);

        // Stage 1: white noise. Stage 2: passive LP "softener".
        let raw = self.noise.next();
        let softened = self.lp_softener.process(raw);

        // Stage 3: Swing-VCA — envelope-shaped excitation.
        let excitation = softened * env;

        // Stage 4: bridged-T resonator (Biquad bandpass) — the drum head's
        // pseudo-tonal body. Excited by enveloped noise, it gives the kick of
        // a real snare without needing a separate body oscillator.
        let body = self.resonator.process(excitation);

        // Stage 5: dry wires layer — softened noise filtered through a HP
        // so it sits on top of the body without muddying the lows.
        let wires_raw = self.wires_hp.process(softened) * env;

        // Mix: tone_mix blends body vs wires; wire_crisp boosts wires HP component.
        let tone = self.tone_mix();
        let crisp = self.wire_crisp();
        let body_gain = 0.4 + tone * 0.6; // 0.4 .. 1.0
        let wires_gain = (1.0 - tone) * 0.5 + crisp * 0.4;

        let mut mixed =
            (body * body_gain + wires_raw * wires_gain) * self.settings.volume * self.drift.level;

        // Apply saturation (post-filter by default)
        mixed = self.saturation.process(mixed);

        self.dc_block_l.process(mixed)
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        if self.settings.stereo < 0.5 {
            let m = self.process_sample();
            return (m, m);
        }

        let env = self.envelope.next();
        if !self.envelope.is_active() {
            self.active = false;
            return (0.0, 0.0);
        }

        // Filter envelope closes the LP softener (×4 depth).
        let filter_env_val = self.filter_env.next();
        let modulated_cutoff = self.settings.filter_freq
            * (1.0 + filter_env_val * self.settings.filter_env_amount * 4.0);
        let cutoff = modulated_cutoff.max(100.0);
        self.lp_softener.set_cutoff(cutoff, self.sample_rate);
        self.lp_softener_r.set_cutoff(cutoff, self.sample_rate);

        // Stage 1-2: independent white noise + LP softener per channel.
        let softened_l = self.lp_softener.process(self.noise.next());
        let softened_r = self.lp_softener_r.process(self.noise_r.next());

        // Stage 3: envelope-shaped excitation.
        let excitation_l = softened_l * env;
        let excitation_r = softened_r * env;

        // Stage 4: bridged-T resonator per channel.
        let body_l = self.resonator.process(excitation_l);
        let body_r = self.resonator_r.process(excitation_r);

        // Stage 5: dry wires layer per channel.
        let wires_l = self.wires_hp.process(softened_l) * env;
        let wires_r = self.wires_hp_r.process(softened_r) * env;

        // Mix.
        let tone = self.tone_mix();
        let crisp = self.wire_crisp();
        let body_gain = 0.4 + tone * 0.6;
        let wires_gain = (1.0 - tone) * 0.5 + crisp * 0.4;
        let vol = self.settings.volume;

        let mut left = (body_l * body_gain + wires_l * wires_gain) * vol * self.drift.level;
        let mut right = (body_r * body_gain + wires_r * wires_gain) * vol * self.drift.level;

        // Apply saturation (post-filter by default)
        left = self.dc_block_l.process(self.saturation.process(left));
        right = self.dc_block_r.process(self.saturation.process(right));

        (left, right)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.envelope.reset();
        self.filter_env.reset();
        self.lp_softener.reset();
        self.lp_softener_r.reset();
        self.wires_hp.reset();
        self.wires_hp_r.reset();
        self.resonator.reset();
        self.resonator_r.reset();
        self.dc_block_l.reset();
        self.dc_block_r.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        let new = Snare606Settings::from(settings);
        let q_changed = (new.resonance - self.settings.resonance).abs() > 1e-4;
        let freq_changed = (new.frequency - self.settings.frequency).abs() > 1e-3;
        let lp_changed = (new.filter_freq - self.settings.filter_freq).abs() > 1e-3;

        self.settings = new;

        if lp_changed {
            self.lp_softener
                .set_cutoff(self.settings.filter_freq.max(500.0), self.sample_rate);
            self.lp_softener_r
                .set_cutoff(self.settings.filter_freq.max(500.0), self.sample_rate);
        }
        if q_changed || freq_changed {
            let q = self.settings.resonance.clamp(0.5, 12.0);
            self.resonator
                .set_bandpass(self.settings.frequency.max(80.0), q, self.sample_rate);
            self.resonator_r
                .set_bandpass(self.settings.frequency.max(80.0), q, self.sample_rate);
        }
        // Update the amp envelope via setters (preserve tail state across
        // retriggers; recreating it reset the value to 0 = a retrigger click).
        self.envelope.set_decay(self.settings.decay);
        self.envelope.set_release(self.settings.release);
        self.envelope.set_decay_curve(self.settings.decay_curve);
        self.envelope.set_release_curve(self.settings.release_curve);
        self.envelope
            .set_attack_ms((self.settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
        self.envelope.set_hold(self.settings.hold);
        self.filter_env
            .set_decay(settings.filter_env_decay.max(0.001));

        // Update saturation config
        self.saturation.saturation_type =
            saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
        self.saturation.pre_filter = self.settings.saturation_pre_filter > 0.5;
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.settings.resonance = value;
            let q = self.resonance_q();
            self.resonator
                .set_bandpass(self.settings.frequency.max(80.0), q, self.sample_rate);
            self.resonator_r
                .set_bandpass(self.settings.frequency.max(80.0), q, self.sample_rate);
        } else if index == 1 {
            self.settings.tone = value;
        } else if index == 2 {
            self.settings.snap = value;
        } else if index == 3 {
            self.settings.saturation_type = value as u8;
            self.saturation.saturation_type =
                saturation::SaturationType::from(self.settings.saturation_type);
        } else if index == 4 {
            self.settings.saturation_amount = value;
            self.saturation.amount = value;
        } else if index == 5 {
            self.settings.saturation_mix = value;
            self.saturation.mix = value;
        } else if index == 6 {
            self.settings.saturation_output_gain = value;
            self.saturation.output_gain = value;
        } else if index == 7 {
            self.settings.saturation_pre_filter = value;
            self.saturation.pre_filter = value > 0.5;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_mode_produces_independent_channels() {
        let mut settings = VoiceSettings::snare606();
        settings.stereo = 1.0;
        let mut voice = Snare606Voice::new(44_100.0, Snare606Settings::from(settings));

        voice.trigger();

        let mut diverged = false;
        for _ in 0..64 {
            let (left, right) = voice.process_sample_stereo();
            if (left - right).abs() > 1e-6 {
                diverged = true;
                break;
            }
        }

        assert!(diverged, "stereo Snare606 should not duplicate mono");
    }
}
