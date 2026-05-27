//! Hi-Hat synthesizer
//!
//! Implementation matches the original Web Audio API:
//! - White noise buffer
//! - Highpass filter (metallic sound)
//! - Short exponential decay (closed hi-hat)

use super::{dsp, settings::hihat::HiHatSettings, Voice, VoiceSettings};

/// Hi-Hat voice using filtered white noise
pub struct HiHatVoice {
    settings: HiHatSettings,
    sample_rate: f32,

    // Noise generators (stereo pair)
    noise: dsp::WhiteNoise,
    noise_r: dsp::WhiteNoise,

    // Peaking filters (stereo pair) — pitch the noise by boosting a narrow band.
    // Frequency controlled by settings.frequency.
    peaking: dsp::Biquad,
    peaking_r: dsp::Biquad,

    // HighPass filters (stereo pair) — cutoff rises after trigger for bright splash then falls.
    // Modulation: cutoff = filter_freq * (1 + filter_env * amount * 1.5)
    filter: dsp::OnePoleFilter,
    filter_r: dsp::OnePoleFilter,

    // Bi-stage amplitude envelope (decay + release).
    envelope: dsp::DecayReleaseEnvelope,
    // Filter envelope for splash decay.
    filter_env: dsp::ExpDecayEnvelope,

    // Active state
    active: bool,
}

impl HiHatVoice {
    pub fn new(sample_rate: f32, settings: HiHatSettings) -> Self {
        let mut peaking = dsp::Biquad::new();
        peaking.set_peaking(settings.frequency, 2.0, 6.0, sample_rate);
        let mut peaking_r = dsp::Biquad::new();
        peaking_r.set_peaking(settings.frequency, 2.0, 6.0, sample_rate);

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
            noise: dsp::WhiteNoise::new(54321),
            noise_r: dsp::WhiteNoise::new(98765),
            peaking,
            peaking_r,
            filter,
            filter_r,
            envelope,
            filter_env: dsp::ExpDecayEnvelope::new(
                sample_rate,
                8.0,
                settings.filter_env_decay.max(0.001),
            )
            .with_attack_ms(0.3),
            active: false,
        }
    }
}

impl Voice for HiHatVoice {
    fn trigger(&mut self) {
        self.active = true;
        // Keep noise generator and filter state continuous across triggers,
        // matching analog drum machine behaviour where the noise source is a
        // free-running zener and the filter is a passive component. The
        // envelope's attack ramp masks any residual transient.
        self.envelope.trigger();
        self.filter_env.trigger();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Apply amplitude envelope
        let env = self.envelope.next();

        let output = match self.settings.algo {
            1 => {
                // Bright: steeper cutoff + slight saturation for extra harmonics
                let filter_env_val = self.filter_env.next();
                let modulated_cutoff = self.settings.filter_freq
                    * 1.5
                    * (1.0 + filter_env_val * self.settings.filter_env_amount * 1.5);
                self.filter
                    .set_cutoff(modulated_cutoff.max(2000.0), self.sample_rate);
                let noise = self.noise.next();
                let peaked = self.peaking.process(noise);
                let filtered = self.filter.process(peaked);
                let saturated = filtered.tanh() * 1.2;
                saturated * env * self.settings.volume
            }
            _ => {
                // Standard: noise + HP
                let filter_env_val = self.filter_env.next();
                let modulated_cutoff = self.settings.filter_freq
                    * (1.0 + filter_env_val * self.settings.filter_env_amount * 1.5);
                self.filter
                    .set_cutoff(modulated_cutoff.max(1000.0), self.sample_rate);
                let noise = self.noise.next();
                let peaked = self.peaking.process(noise);
                let filtered = self.filter.process(peaked);
                filtered * env * self.settings.volume
            }
        };

        // Stop when silent
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

        let env = self.envelope.next();
        let filter_env_val = self.filter_env.next();

        let (cutoff, saturated) = match self.settings.algo {
            1 => {
                let c = self.settings.filter_freq
                    * 1.5
                    * (1.0 + filter_env_val * self.settings.filter_env_amount * 1.5);
                (c.max(2000.0), true)
            }
            _ => {
                let c = self.settings.filter_freq
                    * (1.0 + filter_env_val * self.settings.filter_env_amount * 1.5);
                (c.max(1000.0), false)
            }
        };

        self.filter.set_cutoff(cutoff, self.sample_rate);
        self.filter_r.set_cutoff(cutoff, self.sample_rate);

        let noise_l = self.noise.next();
        let noise_r = self.noise_r.next();
        let peaked_l = self.peaking.process(noise_l);
        let peaked_r = self.peaking_r.process(noise_r);
        let filtered_l = self.filter.process(peaked_l);
        let filtered_r = self.filter_r.process(peaked_r);

        let (left, right) = if saturated {
            (filtered_l.tanh() * 1.2, filtered_r.tanh() * 1.2)
        } else {
            (filtered_l, filtered_r)
        };

        if !self.envelope.is_active() {
            self.active = false;
            return (0.0, 0.0);
        }

        let vol = env * self.settings.volume;
        (left * vol, right * vol)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.peaking.reset();
        self.peaking_r.reset();
        self.filter.reset();
        self.envelope.reset();
        self.filter_env.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = HiHatSettings::from(settings);
        self.peaking
            .set_peaking(self.settings.frequency, 2.0, 6.0, self.sample_rate);
        self.peaking_r
            .set_peaking(self.settings.frequency, 2.0, 6.0, self.sample_rate);
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
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }
    
    fn set_special_param(&mut self, _index: usize, _value: f32) {
        // HiHat has no special parameters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hihat_basic() {
        let mut hihat = HiHatVoice::new(44100.0, HiHatSettings::from(VoiceSettings::hihat()));

        assert!(!hihat.is_active());
        assert_eq!(hihat.process_sample(), 0.0);

        hihat.trigger();
        assert!(hihat.is_active());

        let sample = hihat.process_sample();
        assert!(sample.abs() > 0.0);
    }

    #[test]
    fn test_hihat_short_decay() {
        // Hi-hat should have very short decay
        let settings = VoiceSettings {
            frequency: 8000.0,
            decay: 0.05, // 50ms
            volume: 1.0,
            filter_freq: 10000.0,
            attack: 0.0003,
            release: 0.0, // disable release so the decay test is meaningful
            decay_curve: 8.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.0; 32],
        };
        let mut hihat = HiHatVoice::new(44100.0, HiHatSettings::from(settings));

        hihat.trigger();

        // Process 100ms worth of samples
        for _ in 0..4410 {
            hihat.process_sample();
        }

        // Should be stopped
        assert!(!hihat.is_active());
    }
}
