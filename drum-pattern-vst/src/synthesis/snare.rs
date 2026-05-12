//! Snare drum synthesizer
//!
//! Implementation matches the original Web Audio API:
//! - Triangle oscillator for the "body"
//! - White noise for the "snare" sound
//! - Highpass filter
//! - Exponential amplitude envelope

use super::{Voice, VoiceSettings};

/// Snare drum voice using triangle oscillator + noise
pub struct SnareVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    // Oscillator (triangle) for body
    osc_phase: f32,
    osc_phase_increment: f32,

    // Noise generator
    noise_seed: u32,

    // Highpass filter state
    filter_state: f32,
    filter_alpha: f32,

    // Envelope
    amplitude: f32,
    envelope_value: f32,

    // Active state
    active: bool,
    samples_elapsed: usize,
}

impl SnareVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let phase_increment = settings.frequency / sample_rate;

        // Calculate highpass filter coefficient
        let rc = 1.0 / (2.0 * std::f32::consts::PI * settings.filter_freq);
        let dt = 1.0 / sample_rate;
        let alpha = rc / (rc + dt);

        Self {
            settings,
            sample_rate,
            osc_phase: 0.0,
            osc_phase_increment: phase_increment,
            noise_seed: 12345,
            filter_state: 0.0,
            filter_alpha: alpha,
            amplitude: settings.volume,
            envelope_value: 1.0,
            active: false,
            samples_elapsed: 0,
        }
    }

    /// Generate white noise sample (-1.0 to 1.0)
    fn generate_noise(&mut self) -> f32 {
        // Simple XORShift RNG for white noise
        self.noise_seed ^= self.noise_seed << 13;
        self.noise_seed ^= self.noise_seed >> 17;
        self.noise_seed ^= self.noise_seed << 5;

        // Convert to float range [-1.0, 1.0]
        ((self.noise_seed as f32) / 2147483648.0) - 1.0
    }

    /// Generate triangle wave
    fn generate_triangle(&mut self) -> f32 {
        let tri = if self.osc_phase < 0.5 {
            4.0 * self.osc_phase - 1.0
        } else {
            3.0 - 4.0 * self.osc_phase
        };

        // Update phase
        self.osc_phase += self.osc_phase_increment;
        if self.osc_phase >= 1.0 {
            self.osc_phase -= 1.0;
        }

        tri
    }

    fn calculate_amplitude_envelope(&self, time: f32) -> f32 {
        if time >= self.settings.decay {
            0.01
        } else {
            let decay_factor = (-5.0 * time / self.settings.decay).exp();
            decay_factor.max(0.01)
        }
    }

    fn apply_highpass(&mut self, input: f32) -> f32 {
        let output = self.filter_alpha * (self.filter_state + input);
        self.filter_state = output;
        output
    }
}

impl Voice for SnareVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.samples_elapsed = 0;
        self.osc_phase = 0.0;
        self.envelope_value = 1.0;
        self.filter_state = 0.0;
        // Reset noise seed for consistency
        self.noise_seed = 12345;
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let time = self.samples_elapsed as f32 / self.sample_rate;

        // Mix oscillator and noise (50/50 like original)
        let osc = self.generate_triangle() * 0.5;
        let noise = self.generate_noise() * 0.5;
        let mixed = osc + noise;

        // Apply highpass filter
        let filtered = self.apply_highpass(mixed);

        // Apply amplitude envelope
        self.envelope_value = self.calculate_amplitude_envelope(time);
        let output = filtered * self.envelope_value * self.amplitude;

        // Stop when envelope is too low
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
        self.osc_phase = 0.0;
        self.filter_state = 0.0;
        self.envelope_value = 1.0;
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.amplitude = settings.volume;
        self.osc_phase_increment = settings.frequency / self.sample_rate;
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
    fn test_snare_basic() {
        let mut snare = SnareVoice::new(44100.0, VoiceSettings::snare());

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
        let mut snare = SnareVoice::new(44100.0, settings);

        snare.trigger();

        // Get multiple samples
        let samples: Vec<f32> = (0..100).map(|_| snare.process_sample()).collect();

        // Should have variation (noise component)
        let sum: f32 = samples.iter().sum();
        assert!(sum != 0.0);
    }
}
