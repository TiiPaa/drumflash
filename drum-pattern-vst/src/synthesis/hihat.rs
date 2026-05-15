//! Hi-Hat synthesizer
//!
//! Implementation matches the original Web Audio API:
//! - White noise buffer
//! - Highpass filter (metallic sound)
//! - Short exponential decay (closed hi-hat)

use super::{dsp, Voice, VoiceSettings};

/// Anti-click attack ramp (mimics analog VCA RC charge time).
const HIHAT_ATTACK_MS: f32 = 1.0;

/// Hi-Hat voice using filtered white noise
pub struct HiHatVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    // Noise generator
    noise: dsp::WhiteNoise,

    // Highpass filter
    filter: dsp::OnePoleFilter,

    // Bi-stage amplitude envelope (decay + release).
    envelope: dsp::DecayReleaseEnvelope,

    // Active state
    active: bool,
}

impl HiHatVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);

        let mut envelope = dsp::DecayReleaseEnvelope::new(
            sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms(HIHAT_ATTACK_MS);
        envelope.set_hold(settings.hold);

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(54321),
            filter,
            envelope,
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
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // Generate and filter noise
        let noise = self.noise.next();
        let filtered = self.filter.process(noise);

        // Apply amplitude envelope
        let env = self.envelope.next();
        let output = filtered * env * self.settings.volume;

        // Stop when silent
        if !self.envelope.is_active() {
            self.active = false;
            return 0.0;
        }

        output
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.filter.reset();
        self.envelope.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.filter.set_cutoff(settings.filter_freq, self.sample_rate);
        self.envelope = dsp::DecayReleaseEnvelope::new(
            self.sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms(HIHAT_ATTACK_MS);
        self.envelope.set_hold(settings.hold);
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index < self.settings.special.len() {
            self.settings.special[index] = value;
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hihat_basic() {
        let mut hihat = HiHatVoice::new(44100.0, VoiceSettings::hihat());

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
            release: 0.0, // disable release so the decay test is meaningful
            decay_curve: 8.0,
            release_curve: 3.0,
            hold: 0.0,
            algo: 0,
            special: [0.0; 8],
        };
        let mut hihat = HiHatVoice::new(44100.0, settings);

        hihat.trigger();

        // Process 100ms worth of samples
        for _ in 0..4410 {
            hihat.process_sample();
        }

        // Should be stopped
        assert!(!hihat.is_active());
    }
}
