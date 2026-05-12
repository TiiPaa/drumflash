//! Open hi-hat synthesizer.
//!
//! Similar to the closed hi-hat but with a longer decay and a brighter tail.

use super::{Voice, VoiceSettings};

pub struct OpenHiHatVoice {
    settings: VoiceSettings,
    sample_rate: f32,
    noise_seed: u32,
    filter_state: f32,
    filter_alpha: f32,
    amplitude: f32,
    envelope_value: f32,
    active: bool,
    samples_elapsed: usize,
}

impl OpenHiHatVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * settings.filter_freq);
        let dt = 1.0 / sample_rate;
        let alpha = rc / (rc + dt);

        Self {
            settings,
            sample_rate,
            noise_seed: 24680,
            filter_state: 0.0,
            filter_alpha: alpha,
            amplitude: settings.volume,
            envelope_value: 1.0,
            active: false,
            samples_elapsed: 0,
        }
    }

    fn generate_noise(&mut self) -> f32 {
        self.noise_seed ^= self.noise_seed << 13;
        self.noise_seed ^= self.noise_seed >> 17;
        self.noise_seed ^= self.noise_seed << 5;
        ((self.noise_seed as f32) / 2147483648.0) - 1.0
    }

    fn apply_highpass(&mut self, input: f32) -> f32 {
        let output = self.filter_alpha * (self.filter_state + input);
        self.filter_state = output;
        output
    }

    fn calculate_amplitude_envelope(&self, time: f32) -> f32 {
        if time >= self.settings.decay {
            0.01
        } else {
            let decay_factor = (-5.5 * time / self.settings.decay).exp();
            decay_factor.max(0.01)
        }
    }
}

impl Voice for OpenHiHatVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.samples_elapsed = 0;
        self.filter_state = 0.0;
        self.envelope_value = 1.0;
        self.noise_seed = 24680;
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let time = self.samples_elapsed as f32 / self.sample_rate;
        let noise = self.generate_noise();
        let filtered = self.apply_highpass(noise);
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
        self.filter_state = 0.0;
        self.envelope_value = 1.0;
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.amplitude = settings.volume;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * settings.filter_freq);
        let dt = 1.0 / self.sample_rate;
        self.filter_alpha = rc / (rc + dt);
    }
}
