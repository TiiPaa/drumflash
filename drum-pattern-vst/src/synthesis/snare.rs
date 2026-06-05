//! Snare drum synthesizer
//!
//! Three algorithms:
//! - 0 Synth: triangle osc + noise (original Web Audio model)
//! - 1 Noise: pure white noise
//! - 2 Layered: fundamental + 2nd harmonic + noise
//!
//! For the analog TR-606 bridged-T snare model, see the separate
//! `Snare606Voice` (voice index 10).

use super::{dsp, saturation, settings::snare::SnareSettings, Voice, VoiceSettings};

/// Anti-click floor for the amplitude attack (a true 0 ms attack is a step = click).
const MIN_AMP_ATTACK_MS: f32 = 0.2;

/// Snare drum voice using triangle oscillator + noise
pub struct SnareVoice {
    settings: SnareSettings,
    sample_rate: f32,

    // Oscillator (triangle) for body
    osc: dsp::TriangleOsc,

    // Noise generators (stereo pair)
    noise: dsp::WhiteNoise,
    noise_r: dsp::WhiteNoise,

    // HighPass filters (stereo pair)
    filter: dsp::OnePoleFilter,
    filter_r: dsp::OnePoleFilter,

    // Bi-stage amplitude envelope (decay + release).
    envelope: dsp::DecayReleaseEnvelope,
    // Filter envelope for dynamic snap.
    filter_env: dsp::ExpDecayEnvelope,

    // Saturation stage
    saturation: saturation::SaturationConfig,
    // Per-hit analog drift (breathing) + DC blockers (per channel).
    drift: dsp::AnalogDrift,
    dc_block_l: dsp::DcBlocker,
    dc_block_r: dsp::DcBlocker,

    // Active state
    active: bool,
}

impl SnareVoice {
    pub fn new(sample_rate: f32, settings: SnareSettings) -> Self {
        let mut osc = dsp::TriangleOsc::new(sample_rate);
        osc.set_freq(settings.frequency);

        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);
        let mut filter_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter_r.set_cutoff(settings.filter_freq, sample_rate);

        let mut envelope = dsp::DecayReleaseEnvelope::new(
            sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms((settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
        envelope.set_hold(settings.hold);

        Self {
            settings,
            sample_rate,
            osc,
            noise: dsp::WhiteNoise::new(12345),
            noise_r: dsp::WhiteNoise::new(54321),
            filter,
            filter_r,
            envelope,
            filter_env: dsp::ExpDecayEnvelope::new(
                sample_rate,
                8.0,
                settings.filter_env_decay.max(0.001),
            )
            .with_attack_ms(0.3),
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::None,
                amount: 0.0,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
            },
            drift: dsp::AnalogDrift::new(0x7A7A_3333),
            dc_block_l: dsp::DcBlocker::default(),
            dc_block_r: dsp::DcBlocker::default(),
            active: false,
        }
    }
}

impl Voice for SnareVoice {
    fn trigger(&mut self) {
        let was_active = self.active;
        self.active = true;
        // Cold start only (voice was silent): reset osc phase + filter state for a
        // clean, consistent attack. Never on a ringing-tail retrigger (that jump is
        // the click). The noise generators are always kept continuous.
        if !was_active {
            self.osc.reset();
            self.filter.reset();
            self.filter_r.reset();
            self.dc_block_l.reset();
            self.dc_block_r.reset();
        }
        // analog = per-hit drift (breathing) ; digital = bit-identical hits.
        self.drift.trigger(self.settings.analog >= 0.5);
        self.osc.set_freq(self.settings.frequency * self.drift.pitch);
        self.envelope.set_decay(self.settings.decay * self.drift.time);
        self.envelope
            .set_release(self.settings.release * self.drift.time);
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

        let snap = self.settings.snap;
        let env = self.envelope.next();

        let filter_env_val = self.filter_env.next();
        let modulated_cutoff = self.settings.filter_freq
            * (1.0 + filter_env_val * self.settings.filter_env_amount * 3.0);
        self.filter
            .set_cutoff(modulated_cutoff.max(50.0), self.sample_rate);
        self.filter_r
            .set_cutoff(modulated_cutoff.max(50.0), self.sample_rate);

        let output = match self.settings.algo {
            1 => {
                // Noise: pure white noise, no oscillator
                let mixed = self.noise.next() * 0.5;
                let filtered = self.filter.process(mixed);
                filtered * env * self.settings.volume
            }
            2 => {
                // Layered: fundamental + overtone + noise
                let fundamental = self.osc.next();
                let overtone = ((self.osc.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.3;
                let osc = (fundamental + overtone) * snap * 0.5;
                let noise = self.noise.next() * (1.0 - snap) * 0.5;
                let filtered = self.filter.process(osc + noise);
                filtered * env * self.settings.volume
            }
            _ => {
                // Synth: triangle osc + noise (ratio controlled by snap)
                let osc_gain = snap * 0.5;
                let noise_gain = (1.0 - snap) * 0.5;
                let osc = self.osc.next() * osc_gain;
                let noise = self.noise.next() * noise_gain;
                let filtered = self.filter.process(osc + noise);
                filtered * env * self.settings.volume
            }
        };

        // Stop when envelope is too low
        if !self.envelope.is_active() {
            self.active = false;
            return 0.0;
        }

        self.dc_block_l
            .process(self.saturation.process(output * self.drift.level))
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        if self.settings.stereo < 0.5 {
            let m = self.process_sample();
            return (m, m);
        }

        let snap = self.settings.snap;
        let env = self.envelope.next();

        let filter_env_val = self.filter_env.next();
        let modulated_cutoff = self.settings.filter_freq
            * (1.0 + filter_env_val * self.settings.filter_env_amount * 3.0);
        self.filter
            .set_cutoff(modulated_cutoff.max(50.0), self.sample_rate);
        self.filter_r
            .set_cutoff(modulated_cutoff.max(50.0), self.sample_rate);

        let (left, right) = match self.settings.algo {
            1 => {
                // Noise: pure white noise, no oscillator
                let mixed_l = self.noise.next() * 0.5;
                let mixed_r = self.noise_r.next() * 0.5;
                let filtered_l = self.filter.process(mixed_l);
                let filtered_r = self.filter_r.process(mixed_r);
                (filtered_l, filtered_r)
            }
            2 => {
                // Layered: fundamental + overtone + noise
                let fundamental = self.osc.next();
                let overtone = ((self.osc.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.3;
                let osc = (fundamental + overtone) * snap * 0.5;
                let noise_l = self.noise.next() * (1.0 - snap) * 0.5;
                let noise_r = self.noise_r.next() * (1.0 - snap) * 0.5;
                let filtered_l = self.filter.process(osc + noise_l);
                let filtered_r = self.filter_r.process(osc + noise_r);
                (filtered_l, filtered_r)
            }
            _ => {
                // Synth: triangle osc + noise (ratio controlled by snap)
                let osc_gain = snap * 0.5;
                let noise_gain = (1.0 - snap) * 0.5;
                let osc = self.osc.next() * osc_gain;
                let noise_l = self.noise.next() * noise_gain;
                let noise_r = self.noise_r.next() * noise_gain;
                let filtered_l = self.filter.process(osc + noise_l);
                let filtered_r = self.filter_r.process(osc + noise_r);
                (filtered_l, filtered_r)
            }
        };

        if !self.envelope.is_active() {
            self.active = false;
            return (0.0, 0.0);
        }

        let vol = env * self.settings.volume * self.drift.level;
        let l = self.dc_block_l.process(self.saturation.process(left * vol));
        let r = self.dc_block_r.process(self.saturation.process(right * vol));
        (l, r)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.osc.reset();
        self.filter.reset();
        self.filter_r.reset();
        self.envelope.reset();
        self.filter_env.reset();
        self.dc_block_l.reset();
        self.dc_block_r.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = SnareSettings::from(settings);
        self.osc.set_freq(self.settings.frequency);
        self.filter
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.filter_r
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
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
            .set_decay(self.settings.filter_env_decay.max(0.001));
        self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
        self.saturation.pre_filter = self.settings.saturation_pre_filter > 0.5;
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }
    
    fn set_special_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.settings.snap = value,
            1 => {
                self.settings.saturation_type = value as u8;
                self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
            }
            2 => {
                self.settings.saturation_amount = value;
                self.saturation.amount = value;
            }
            3 => {
                self.settings.saturation_mix = value;
                self.saturation.mix = value;
            }
            4 => {
                self.settings.saturation_output_gain = value;
                self.saturation.output_gain = value;
            }
            5 => {
                self.settings.saturation_pre_filter = value;
                self.saturation.pre_filter = value > 0.5;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snare_basic() {
        let mut snare = SnareVoice::new(44100.0, SnareSettings::from(VoiceSettings::snare()));

        // Silent before trigger
        assert!(!snare.is_active());
        assert_eq!(snare.process_sample(), 0.0);

        // Trigger
        snare.trigger();
        assert!(snare.is_active());

        // Should produce sound
        let sample = snare.process_sample();
        assert!(sample.abs() > 0.0);
    }

    #[test]
    fn test_snare_has_noise() {
        let settings = VoiceSettings::snare();
        let mut snare = SnareVoice::new(44100.0, SnareSettings::from(settings));

        snare.trigger();

        // Get multiple samples
        let samples: Vec<f32> = (0..100).map(|_| snare.process_sample()).collect();

        // Should have variation (noise component)
        let sum: f32 = samples.iter().sum();
        assert!(sum != 0.0);
    }
}
