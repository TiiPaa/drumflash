//! Hi-Hat synthesizer
//!
//! Implementation matches the original Web Audio API:
//! - White noise buffer
//! - Highpass filter (metallic sound)
//! - Short exponential decay (closed hi-hat)

use super::{Voice, VoiceSettings};

/// Hi-Hat voice using filtered white noise
pub struct HiHatVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    // Noise generator
    noise_seed: u32,

    // Simple highpass filter state (one-pole)
    filter_state: f32,
    filter_alpha: f32, // Filter coefficient

    // Envelope
    amplitude: f32,
    envelope_value: f32,

    // Active state
    active: bool,
    samples_elapsed: usize,
}

impl HiHatVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        // Calculate highpass filter coefficient
        // Simple first-order highpass: y[n] = alpha * (y[n-1] + x[n] - x[n-1])
        // where alpha = 1 / (1 + 2*PI*fc/fs)
        let rc = 1.0 / (2.0 * std::f32::consts::PI * settings.filter_freq);
        let dt = 1.0 / sample_rate;
        let alpha = rc / (rc + dt);

        Self {
            settings,
            sample_rate,
            noise_seed: 54321,
            filter_state: 0.0,
            filter_alpha: alpha,
            amplitude: settings.volume,
            envelope_value: 1.0,
            active: false,
            samples_elapsed: 0,
        }
    }

    /// Generate white noise sample
    fn generate_noise(&mut self) -> f32 {
        self.noise_seed ^= self.noise_seed << 13;
        self.noise_seed ^= self.noise_seed >> 17;
        self.noise_seed ^= self.noise_seed << 5;
        ((self.noise_seed as f32) / 2147483648.0) - 1.0
    }

    /// Apply simple highpass filter
    fn apply_highpass(&mut self, input: f32) -> f32 {
        // First-order highpass filter
        let output = self.filter_alpha * (self.filter_state + input);
        self.filter_state = output;
        output
    }

    fn calculate_amplitude_envelope(&self, time: f32) -> f32 {
        if time >= self.settings.decay {
            0.01
        } else {
            // Steeper decay for metallic sound
            let decay_factor = (-8.0 * time / self.settings.decay).exp();
            decay_factor.max(0.01)
        }
    }
}

impl Voice for HiHatVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.samples_elapsed = 0;
        self.filter_state = 0.0;
        self.envelope_value = 1.0;
        self.noise_seed = 54321;
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let time = self.samples_elapsed as f32 / self.sample_rate;

        // Generate and filter noise
        let noise = self.generate_noise();
        let filtered = self.apply_highpass(noise);

        // Apply amplitude envelope
        self.envelope_value = self.calculate_amplitude_envelope(time);
        let output = filtered * self.envelope_value * self.amplitude;

        // Stop when silent
        if self.envelope_value <= 0.01 && time >= self.settings.decay {
            self.active = false;
            return 0.0;
        }

        self.samples_elapsed += 1;
        output
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.samples_elapsed = 0;
        self.filter_state = 0.0;
        self.envelope_value = 1.0;
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.amplitude = settings.volume;
        // Update filter coefficient
        let rc = 1.0 / (2.0 * std::f32::consts::PI * settings.filter_freq);
        let dt = 1.0 / self.sample_rate;
        self.filter_alpha = rc / (rc + dt);
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
