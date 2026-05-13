//! Kick drum synthesizer — refactored with shared DSP primitives.
//!
//! Architecture:
//! - Sine oscillator with exponential pitch-drop envelope
//! - One-pole lowpass filter
//! - Exponential amplitude envelope
//! - Optional click transient (impulse + short noise burst)

use super::{dsp, special_params, AlgoDef, SpecialParamDef, Voice, VoiceSettings};

pub struct KickVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    osc_sine: dsp::SineOsc,
    osc_square: dsp::SquareOsc,
    fm_carrier: dsp::SineOsc,
    fm_mod: dsp::SineOsc,
    pitch_env: dsp::PitchEnvelope,
    filter: dsp::OnePoleFilter,
    amp_env: dsp::ExpDecayEnvelope,
    click: dsp::ClickGenerator,

    active: bool,
}

impl KickVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let mut osc_sine = dsp::SineOsc::new(sample_rate);
        osc_sine.set_freq(settings.frequency);

        let mut osc_square = dsp::SquareOsc::new(sample_rate);
        osc_square.set_freq(settings.frequency);

        let mut fm_carrier = dsp::SineOsc::new(sample_rate);
        fm_carrier.set_freq(settings.frequency);

        let mut fm_mod = dsp::SineOsc::new(sample_rate);
        fm_mod.set_freq(settings.frequency * 0.5);

        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);

        let mut voice = Self {
            settings,
            sample_rate,
            osc_sine,
            osc_square,
            fm_carrier,
            fm_mod,
            pitch_env: dsp::PitchEnvelope::new(sample_rate, 1.0, 0.3, 0.12),
            filter,
            amp_env: dsp::ExpDecayEnvelope::new(sample_rate, 5.0, settings.decay),
            click: dsp::ClickGenerator::new(sample_rate, 10.0, 0.3, 1.0),
            active: false,
        };
        voice.update_derived_params();
        voice
    }

    fn update_derived_params(&mut self) {
        self.osc_sine.set_freq(self.settings.frequency);
        self.osc_square.set_freq(self.settings.frequency);
        self.fm_carrier.set_freq(self.settings.frequency);
        self.fm_mod.set_freq(self.settings.frequency * 0.5);
        self.filter.set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        // Pitch envelope: start at fundamental, drop to 30% over 0.12 s
        self.pitch_env = dsp::PitchEnvelope::new(
            self.sample_rate,
            1.0,
            0.3,
            0.12,
        );
    }

    fn click_amount(&self) -> f32 {
        self.settings.special[0]
    }
}

impl Voice for KickVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.osc_sine.reset();
        self.osc_square.reset();
        self.fm_carrier.reset();
        self.fm_mod.reset();
        self.filter.reset();
        self.pitch_env.trigger();
        self.amp_env.trigger();
        if self.click_amount() > 0.0 {
            self.click.trigger();
        }
    }

    fn process_sample(&mut self) -> f32 {
        let mut body = 0.0f32;

        if self.active {
            // Pitch sweep
            let pitch_ratio = self.pitch_env.next();
            let base_freq = self.settings.frequency * pitch_ratio;

            // Generate based on selected algorithm
            let raw = match self.settings.algo {
                1 => {
                    // Square: more harmonics, brighter
                    self.osc_square.set_freq(base_freq);
                    self.osc_square.next()
                }
                2 => {
                    // FM: punchy, complex
                    self.fm_mod.set_freq(base_freq * 0.5);
                    let mod_val = self.fm_mod.next();
                    self.fm_carrier.set_freq(base_freq * (1.0 + mod_val * 0.8));
                    self.fm_carrier.next()
                }
                _ => {
                    // Sine: classic round kick
                    self.osc_sine.set_freq(base_freq);
                    self.osc_sine.next()
                }
            };

            let filtered = self.filter.process(raw);

            // Amplitude envelope
            let env = self.amp_env.next();
            if env <= 0.0 {
                self.active = false;
            } else {
                body = filtered * env * self.settings.volume;
            }
        }

        // Click transient — allowed to ring out even if body finished
        let click = if self.click_amount() > 0.0 && self.click.is_active() {
            self.click.next() * self.click_amount()
        } else {
            0.0
        };

        body + click
    }

    fn is_active(&self) -> bool {
        self.active || self.click.is_active()
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
        self.click.reset();
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
        special_params::KICK_ALGOS
    }

    fn special_params(&self) -> &'static [SpecialParamDef] {
        special_params::KICK_SPECIALS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kick_basic() {
        let mut kick = KickVoice::new(44100.0, VoiceSettings::kick());

        assert!(!kick.is_active());
        assert_eq!(kick.process_sample(), 0.0);

        kick.trigger();
        assert!(kick.is_active());

        let has_signal = (0..8).any(|_| kick.process_sample().abs() > 0.0);
        assert!(has_signal);
    }

    #[test]
    fn test_kick_click() {
        let mut kick = KickVoice::new(44100.0, VoiceSettings::kick());
        kick.set_special_param(0, 1.0);
        kick.trigger();

        // First sample should contain click energy (impulse + noise, level 1.0)
        let first = kick.process_sample().abs();
        assert!(first > 0.2, "Click should produce strong first sample: {}", first);
    }

    #[test]
    fn test_kick_click_at_zero_is_silent() {
        let mut kick = KickVoice::new(44100.0, VoiceSettings::kick());
        kick.set_special_param(0, 0.0);
        kick.trigger();

        // First sample: body starts at sin(0)=0, filter is reset, click is OFF
        let first = kick.process_sample().abs();
        assert!(first < 0.0001, "Click should be silent at amount=0: {}", first);
    }

    #[test]
    fn test_kick_decay() {
        let settings = VoiceSettings {
            frequency: 60.0,
            decay: 0.01,
            volume: 1.0,
            filter_freq: 100.0,
            algo: 0,
            special: [0.0; 8],
        };
        let mut kick = KickVoice::new(44100.0, settings);

        kick.trigger();
        for _ in 0..1000 {
            kick.process_sample();
        }
        assert!(!kick.is_active());
    }
}
