//! Tom synthesizer — refactored with shared DSP primitives.
//!
//! Architecture:
//! - Sine oscillator + 2nd harmonic overtone (×0.22)
//! - Exponential pitch sweep to 55 % of fundamental
//! - One-pole lowpass filter
//! - Exponential amplitude envelope
//! - Optional stick attack (short noise burst) for realism

use super::{dsp, settings::tom::TomSettings, Voice, VoiceSettings};

pub struct TomVoice {
    settings: TomSettings,
    sample_rate: f32,

    osc: dsp::SineOsc,
    pitch_env: dsp::PitchEnvelope,
    // LowPass filter — cutoff closes after trigger for natural damp.
    // Modulation: cutoff = filter_freq * (1 + filter_env * amount * 4.0)
    filter: dsp::OnePoleFilter,
    amp_env: dsp::DecayReleaseEnvelope,
    // Filter envelope for natural "bouum" decay.
    filter_env: dsp::ExpDecayEnvelope,
    stick_attack: dsp::ClickGenerator,

    active: bool,
}

impl TomVoice {
    pub fn new(sample_rate: f32, settings: TomSettings) -> Self {
        let sweep_time = 0.14f32.min(settings.decay);
        let mut osc = dsp::SineOsc::new(sample_rate);
        osc.set_freq(settings.frequency);

        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);

        let mut voice = Self {
            settings,
            sample_rate,
            osc,
            pitch_env: dsp::PitchEnvelope::new(sample_rate, 1.0, 0.55, sweep_time),
            filter,
            amp_env: dsp::DecayReleaseEnvelope::new(
                sample_rate,
                settings.decay_curve,
                settings.decay,
                settings.release_curve,
                settings.release,
            )
            .with_attack_ms(settings.attack * 1000.0),
            filter_env: dsp::ExpDecayEnvelope::new(
                sample_rate,
                6.0,
                settings.filter_env_decay.max(0.001),
            )
            .with_attack_ms(0.5),
            stick_attack: dsp::ClickGenerator::new(sample_rate, 8.0, 0.5, 0.6),
            active: false,
        };
        voice.update_derived_params();
        voice
    }

    fn update_derived_params(&mut self) {
        self.osc.set_freq(self.settings.frequency);
        self.filter
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        self.amp_env.set_attack_ms(self.settings.attack * 1000.0);
        self.amp_env.set_release(self.settings.release);
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
        self.filter_env
            .set_decay(self.settings.filter_env_decay.max(0.001));
        let sweep_time = 0.14f32.min(self.settings.decay);
        self.pitch_env = dsp::PitchEnvelope::new(self.sample_rate, 1.0, 0.55, sweep_time);
    }

    fn stick_amount(&self) -> f32 {
        self.settings.stick_attack
    }
}

impl Voice for TomVoice {
    fn trigger(&mut self) {
        self.active = true;
        if self.settings.analog < 0.5 {
            // Digital stable: reset phase and filter state for identical hits.
            self.osc.phase = 0.0;
            self.filter.reset();
        }
        // Analog-style retrigger: keep oscillator phase and filter state intact.
        // See kick.rs for the rationale — tonal voices click hard on a phase reset
        // when retriggered during a ringing tail.
        self.pitch_env.trigger();
        self.amp_env.trigger();
        self.filter_env.trigger();
        if self.stick_amount() > 0.0 {
            self.stick_attack.trigger();
        }
    }

    fn process_sample(&mut self) -> f32 {
        let mut tone = 0.0f32;

        if self.active {
            // Pitch sweep
            let pitch_ratio = self.pitch_env.next();
            self.osc.set_freq(self.settings.frequency * pitch_ratio);

            // Amplitude envelope
            let env = self.amp_env.next();
            if env <= 0.0 {
                self.active = false;
            } else {
                let (body, modulated_cutoff) = match self.settings.algo {
                    1 => {
                        // Deep: lower pitch, darker tone, less overtone
                        let pitch_ratio = self.pitch_env.next();
                        let deep_freq = self.settings.frequency * 0.7;
                        self.osc.set_freq(deep_freq * pitch_ratio);
                        let fundamental = self.osc.next();
                        let overtone =
                            ((self.osc.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.12;
                        let filter_env_val = self.filter_env.next();
                        let cutoff = self.settings.filter_freq
                            * 0.7
                            * (1.0 + filter_env_val * self.settings.filter_env_amount * 4.0);
                        (fundamental + overtone, cutoff.max(50.0))
                    }
                    _ => {
                        // Standard: sine + overtone, pitch sweep
                        let pitch_ratio = self.pitch_env.next();
                        self.osc.set_freq(self.settings.frequency * pitch_ratio);
                        let fundamental = self.osc.next();
                        let overtone =
                            ((self.osc.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.22;
                        let filter_env_val = self.filter_env.next();
                        let cutoff = self.settings.filter_freq
                            * (1.0 + filter_env_val * self.settings.filter_env_amount * 4.0);
                        (fundamental + overtone, cutoff.max(100.0))
                    }
                };
                self.filter.set_cutoff(modulated_cutoff, self.sample_rate);
                let filtered = self.filter.process(body);
                tone = filtered * env * self.settings.volume;
            }
        }

        // Stick attack — allowed to ring out even if body finished
        let attack = if self.stick_amount() > 0.0 && self.stick_attack.is_active() {
            self.stick_attack.next() * self.stick_amount()
        } else {
            0.0
        };

        tone + attack
    }

    fn is_active(&self) -> bool {
        self.active || self.stick_attack.is_active()
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
        self.filter_env.reset();
        self.stick_attack.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = TomSettings::from(settings);
        self.update_derived_params();
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
        self.update_derived_params();
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.settings.stick_attack = value;
        }
    }
}
