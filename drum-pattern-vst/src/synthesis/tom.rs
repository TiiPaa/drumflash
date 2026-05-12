//! Tom synthesizer.
//!
//! A tom is closer to the kick family than to the snare, but with a gentler pitch sweep
//! and a more sustained body.

use super::{Voice, VoiceSettings};

pub struct TomVoice {
    settings: VoiceSettings,
    sample_rate: f32,
    phase: f32,
    phase_increment: f32,
    current_frequency: f32,
    filter_state: f32,
    filter_alpha: f32,
    amplitude: f32,
    envelope_value: f32,
    active: bool,
    samples_elapsed: usize,
}

impl TomVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let filter_alpha = Self::calculate_filter_alpha(sample_rate, settings.filter_freq);
        Self {
            settings,
            sample_rate,
            phase: 0.0,
            phase_increment: settings.frequency / sample_rate,
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
        let sweep_time = 0.14f32.min(self.settings.decay);
        if time >= sweep_time {
            self.settings.frequency * 0.55
        } else {
            let t = time / sweep_time.max(0.001);
            let start = self.settings.frequency;
            let end = self.settings.frequency * 0.55;
            start * (end / start).powf(t)
        }
    }

    fn calculate_amplitude_envelope(&self, time: f32) -> f32 {
        if time >= self.settings.decay {
            0.01
        } else {
            let decay_factor = (-4.2 * time / self.settings.decay).exp();
            decay_factor.max(0.01)
        }
    }
}

impl Voice for TomVoice {
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
        self.current_frequency = self.calculate_pitch_envelope(time);
        self.phase_increment = self.current_frequency / self.sample_rate;

        let fundamental = (self.phase * 2.0 * std::f32::consts::PI).sin();
        let overtone = ((self.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.22;
        let body = fundamental + overtone;

        // Apply lowpass filter
        let filtered = self.filter_alpha * body + (1.0 - self.filter_alpha) * self.filter_state;
        self.filter_state = filtered;

        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        self.envelope_value = self.calculate_amplitude_envelope(time);
        let output = filtered * self.envelope_value * self.amplitude;

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
