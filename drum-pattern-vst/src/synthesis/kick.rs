//! Kick drum synthesizer
//!
//! Implementation matches the original Web Audio API approach:
//! - Sine oscillator with exponential pitch drop
//! - Lowpass filter
//! - Exponential amplitude envelope

use super::{Voice, VoiceSettings};

/// Kick drum voice using sine oscillator with pitch envelope
pub struct KickVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    // Oscillator state
    phase: f32,
    phase_increment: f32,
    current_frequency: f32,

    // Lowpass filter state
    filter_state: f32,
    filter_alpha: f32,

    // Envelope state
    amplitude: f32,
    envelope_value: f32,

    // Is voice active?
    active: bool,
    samples_elapsed: usize,
}

impl KickVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let phase_increment = settings.frequency / sample_rate;
        let filter_alpha = Self::calculate_filter_alpha(sample_rate, settings.filter_freq);

        Self {
            settings,
            sample_rate,
            phase: 0.0,
            phase_increment,
            current_frequency: settings.frequency,
            filter_state: 0.0,
            filter_alpha,
            amplitude: settings.volume,
            envelope_value: 1.0,
            active: false,
            samples_elapsed: 0,
        }
    }

    fn calculate_filter_alpha(sample_rate: f32, filter_freq: f32) -> f32 {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * filter_freq.max(1.0));
        let dt = 1.0 / sample_rate;
        dt / (rc + dt)
    }

    fn calculate_pitch_envelope(&self, time: f32) -> f32 {
        // Exponential ramp from frequency to frequency * 0.1 over 0.1 seconds
        // Matches the Web Audio: osc.frequency.exponentialRampToValueAtTime(freq * 0.1, now + 0.1)
        if time >= 0.1 {
            self.settings.frequency * 0.1
        } else {
            let t = time / 0.1;
            let start = self.settings.frequency;
            let end = self.settings.frequency * 0.1;
            // Exponential interpolation
            start * (end / start).powf(t)
        }
    }

    fn calculate_amplitude_envelope(&self, time: f32) -> f32 {
        // Exponential decay envelope
        // gain.gain.exponentialRampToValueAtTime(0.01, now + decay)
        if time >= self.settings.decay {
            0.01 // Silent enough to stop
        } else {
            let decay_factor = (-5.0 * time / self.settings.decay).exp();
            decay_factor.max(0.01)
        }
    }
}

impl Voice for KickVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.samples_elapsed = 0;
        self.phase = 0.0;
        self.current_frequency = self.settings.frequency;
        self.filter_state = 0.0;
        self.envelope_value = 1.0;
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let time = self.samples_elapsed as f32 / self.sample_rate;

        // Update frequency based on pitch envelope
        let target_freq = self.calculate_pitch_envelope(time);
        self.current_frequency = target_freq;
        self.phase_increment = self.current_frequency / self.sample_rate;

        // Generate sine wave
        let sine = (self.phase * 2.0 * std::f32::consts::PI).sin();

        // Apply lowpass filter
        let filtered = self.filter_alpha * sine + (1.0 - self.filter_alpha) * self.filter_state;
        self.filter_state = filtered;

        // Update phase
        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Apply amplitude envelope
        self.envelope_value = self.calculate_amplitude_envelope(time);
        let output = filtered * self.envelope_value * self.amplitude;

        // Check if we should stop
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
        self.phase = 0.0;
        self.filter_state = 0.0;
        self.envelope_value = 1.0;
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.amplitude = settings.volume;
        self.filter_alpha = Self::calculate_filter_alpha(self.sample_rate, settings.filter_freq);
        if !self.active {
            self.current_frequency = settings.frequency;
            self.phase_increment = settings.frequency / self.sample_rate;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kick_basic() {
        let mut kick = KickVoice::new(44100.0, VoiceSettings::kick());

        // Should be silent before trigger
        assert!(!kick.is_active());
        assert_eq!(kick.process_sample(), 0.0);

        // Trigger and check it becomes active
        kick.trigger();
        assert!(kick.is_active());

        // The sine oscillator starts at phase zero, so the first sample can be silent.
        let has_signal = (0..8).any(|_| kick.process_sample().abs() > 0.0);
        assert!(has_signal);
    }

    #[test]
    fn test_kick_decay() {
        let settings = VoiceSettings {
            frequency: 60.0,
            decay: 0.01, // Very short decay for testing
            volume: 1.0,
            filter_freq: 100.0,
        };
        let mut kick = KickVoice::new(44100.0, settings);

        kick.trigger();

        // Process enough samples to exceed decay time
        for _ in 0..1000 {
            kick.process_sample();
        }

        // Should have stopped
        assert!(!kick.is_active());
    }
}
