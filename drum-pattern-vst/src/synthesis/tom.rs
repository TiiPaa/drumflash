//! Tom synthesizer — refactored with shared DSP primitives.
//!
//! Architecture:
//! - Sine oscillator + 2nd harmonic overtone (×0.22)
//! - Exponential pitch sweep to 55 % of fundamental
//! - One-pole lowpass filter
//! - Exponential amplitude envelope
//! - Optional stick attack (short noise burst) for realism

use super::{dsp, special_params, AlgoDef, SpecialParamDef, Voice, VoiceSettings};

pub struct TomVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    osc: dsp::SineOsc,
    pitch_env: dsp::PitchEnvelope,
    filter: dsp::OnePoleFilter,
    amp_env: dsp::ExpDecayEnvelope,
    stick_attack: dsp::ClickGenerator,

    active: bool,
}

impl TomVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
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
            amp_env: dsp::ExpDecayEnvelope::new(sample_rate, 4.2, settings.decay)
                .with_attack_ms(1.5),
            stick_attack: dsp::ClickGenerator::new(sample_rate, 8.0, 0.5, 0.6),
            active: false,
        };
        voice.update_derived_params();
        voice
    }

    fn update_derived_params(&mut self) {
        self.osc.set_freq(self.settings.frequency);
        self.filter.set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        let sweep_time = 0.14f32.min(self.settings.decay);
        self.pitch_env = dsp::PitchEnvelope::new(self.sample_rate, 1.0, 0.55, sweep_time);
    }

    fn stick_amount(&self) -> f32 {
        self.settings.special[0]
    }
}

impl Voice for TomVoice {
    fn trigger(&mut self) {
        self.active = true;
        // Analog-style retrigger: keep oscillator phase and filter state intact.
        // See kick.rs for the rationale — tonal voices click hard on a phase reset
        // when retriggered during a ringing tail.
        self.pitch_env.trigger();
        self.amp_env.trigger();
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

            // Fundamental + 2nd harmonic overtone
            let fundamental = self.osc.next();
            let overtone = ((self.osc.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.22;
            let body = fundamental + overtone;

            // Filter
            let filtered = self.filter.process(body);

            // Amplitude envelope
            let env = self.amp_env.next();
            if env <= 0.0 {
                self.active = false;
            } else {
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
        self.stick_attack.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.update_derived_params();
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
        self.update_derived_params();
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index < self.settings.special.len() {
            self.settings.special[index] = value;
        }
    }

    fn supported_algos(&self) -> &'static [AlgoDef] {
        special_params::TOM_ALGOS
    }

    fn special_params(&self) -> &'static [SpecialParamDef] {
        special_params::TOM_SPECIALS
    }
}
