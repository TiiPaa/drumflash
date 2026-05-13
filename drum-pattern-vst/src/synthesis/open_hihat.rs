//! Open hi-hat synthesizer.
//!
//! Similar to the closed hi-hat but with a longer decay and a brighter tail.

use super::{dsp, special_params, AlgoDef, SpecialParamDef, Voice, VoiceSettings};

pub struct OpenHiHatVoice {
    settings: VoiceSettings,
    sample_rate: f32,
    noise: dsp::WhiteNoise,
    filter: dsp::OnePoleFilter,
    envelope: dsp::ExpDecayEnvelope,
    active: bool,
    samples_elapsed: usize,
}

impl OpenHiHatVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);

        let envelope = dsp::ExpDecayEnvelope::new(
            sample_rate,
            5.5 / settings.decay.max(0.001),
            settings.decay,
        );

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(24680),
            filter,
            envelope,
            active: false,
            samples_elapsed: 0,
        }
    }
}

impl Voice for OpenHiHatVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.samples_elapsed = 0;
        self.noise.reseed(24680);
        self.filter.reset();
        self.envelope.trigger();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let noise = self.noise.next();
        let filtered = self.filter.process(noise);
        let env = self.envelope.next().max(0.01);

        let time = self.samples_elapsed as f32 / self.sample_rate;
        let output = filtered * env * self.settings.volume;
        self.samples_elapsed += 1;

        if env <= 0.01 && time >= self.settings.decay {
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
        self.samples_elapsed = 0;
        self.filter.reset();
        self.envelope.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.filter.set_cutoff(settings.filter_freq, self.sample_rate);
        self.envelope = dsp::ExpDecayEnvelope::new(
            self.sample_rate,
            5.5 / settings.decay.max(0.001),
            settings.decay,
        );
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index < self.settings.special.len() {
            self.settings.special[index] = value;
        }
    }

    fn supported_algos(&self) -> &'static [AlgoDef] {
        special_params::HIHAT_ALGOS
    }

    fn special_params(&self) -> &'static [SpecialParamDef] {
        special_params::HIHAT_SPECIALS
    }
}
