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
        .with_attack_ms(settings.attack * 1000.0);
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
            active: false,
        }
    }
}

impl Voice for SnareVoice {
    fn trigger(&mut self) {
        self.active = true;
        if self.settings.analog < 0.5 {
            // Digital stable: reset phase and filter state for identical hits.
            self.osc.reset();
            self.filter.reset();
        }
        // Keep oscillator phase, noise generator and filter state continuous
        // across triggers — see kick.rs for the rationale.
        self.envelope.trigger();
        self.filter_env.trigger();
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

        let raw = match self.settings.algo {
            1 => {
                // Noise: pure white noise, no oscillator
                self.noise.next() * 0.5
            }
            2 => {
                // Layered: fundamental + overtone + noise
                let fundamental = self.osc.next();
                let overtone = ((self.osc.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.3;
                let osc = (fundamental + overtone) * snap * 0.5;
                let noise = self.noise.next() * (1.0 - snap) * 0.5;
                osc + noise
            }
            _ => {
                // Synth: triangle osc + noise (ratio controlled by snap)
                let osc_gain = snap * 0.5;
                let noise_gain = (1.0 - snap) * 0.5;
                let osc = self.osc.next() * osc_gain;
                let noise = self.noise.next() * noise_gain;
                osc + noise
            }
        };

        // Apply saturation pre-filter (on the raw source before filtering)
        let saturated_raw = if self.saturation.pre_filter {
            self.saturation.process(raw)
        } else {
            raw
        };

        let filtered = self.filter.process(saturated_raw);
        let mut output = filtered * env * self.settings.volume;

        // Apply saturation post-filter (after the full signal chain)
        if !self.saturation.pre_filter {
            output = self.saturation.process(output);
        }

        // Stop when envelope is too low
        if !self.envelope.is_active() {
            self.active = false;
            return 0.0;
        }

        output
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

        let (raw_l, raw_r) = match self.settings.algo {
            1 => {
                // Noise: pure white noise, no oscillator
                (self.noise.next() * 0.5, self.noise_r.next() * 0.5)
            }
            2 => {
                // Layered: fundamental + overtone + noise
                let fundamental = self.osc.next();
                let overtone = ((self.osc.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.3;
                let osc = (fundamental + overtone) * snap * 0.5;
                let noise_l = self.noise.next() * (1.0 - snap) * 0.5;
                let noise_r = self.noise_r.next() * (1.0 - snap) * 0.5;
                (osc + noise_l, osc + noise_r)
            }
            _ => {
                // Synth: triangle osc + noise (ratio controlled by snap)
                let osc_gain = snap * 0.5;
                let noise_gain = (1.0 - snap) * 0.5;
                let osc = self.osc.next() * osc_gain;
                let noise_l = self.noise.next() * noise_gain;
                let noise_r = self.noise_r.next() * noise_gain;
                (osc + noise_l, osc + noise_r)
            }
        };

        // Apply saturation pre-filter (on the raw source before filtering)
        let (saturated_l, saturated_r) = if self.saturation.pre_filter {
            (self.saturation.process(raw_l), self.saturation.process(raw_r))
        } else {
            (raw_l, raw_r)
        };

        let filtered_l = self.filter.process(saturated_l);
        let filtered_r = self.filter_r.process(saturated_r);

        if !self.envelope.is_active() {
            self.active = false;
            return (0.0, 0.0);
        }

        let vol = env * self.settings.volume;
        let mut left = filtered_l * vol;
        let mut right = filtered_r * vol;

        // Apply saturation post-filter (after the full signal chain)
        if !self.saturation.pre_filter {
            left = self.saturation.process(left);
            right = self.saturation.process(right);
        }

        (left, right)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.osc.reset();
        self.filter.reset();
        self.envelope.reset();
        self.filter_env.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = SnareSettings::from(settings);
        self.osc.set_freq(self.settings.frequency);
        self.filter
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.filter_r
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.envelope = dsp::DecayReleaseEnvelope::new(
            self.sample_rate,
            self.settings.decay_curve,
            self.settings.decay,
            self.settings.release_curve,
            self.settings.release,
        )
        .with_attack_ms(self.settings.attack * 1000.0);
        self.envelope.set_hold(self.settings.hold);
        self.filter_env
            .set_decay(self.settings.filter_env_decay.max(0.001));

        // Update saturation config
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
        if index == 0 {
            self.settings.snap = value;
        } else if index == 1 {
            self.settings.saturation_type = value as u8;
            self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
        } else if index == 2 {
            self.settings.saturation_amount = value;
            self.saturation.amount = value;
        } else if index == 3 {
            self.settings.saturation_mix = value;
            self.saturation.mix = value;
        } else if index == 4 {
            self.settings.saturation_output_gain = value;
            self.saturation.output_gain = value;
        } else if index == 5 {
            self.settings.saturation_pre_filter = value;
            self.saturation.pre_filter = value > 0.5;
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
