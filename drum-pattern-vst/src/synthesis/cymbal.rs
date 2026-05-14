//! Crash cymbal synthesizer.
//!
//! Architecture:
//! - Dense white noise
//! - Highpass filter for brightness and "wash"
//! - Very long exponential decay
//! - Slight pitch modulation (FM) for the shimmering wash effect

use super::{dsp, special_params, AlgoDef, SpecialParamDef, Voice, VoiceSettings};

pub struct CymbalVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    noise: dsp::WhiteNoise,
    filter: dsp::OnePoleFilter,
    amp_env: dsp::DecayReleaseEnvelope,

    // FM shimmer state
    fm_phase: f32,
    fm_increment: f32,

    active: bool,
}

impl CymbalVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq.max(4000.0), sample_rate);

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(0xDEAD_BEEF),
            filter,
            amp_env: dsp::DecayReleaseEnvelope::new(
                sample_rate,
                settings.decay_curve,
                settings.decay,
                settings.release_curve,
                settings.release,
            )
            .with_attack_ms(2.0),
            fm_phase: 0.0,
            fm_increment: 15.0 / sample_rate, // 15 Hz modulation
            active: false,
        }
    }

    fn update_derived_params(&mut self) {
        self.filter.set_cutoff(self.settings.filter_freq.max(4000.0), self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        self.amp_env.set_release(self.settings.release);
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
    }
}

impl Voice for CymbalVoice {
    fn trigger(&mut self) {
        self.active = true;
        // Keep filter state and FM LFO phase continuous across triggers.
        self.amp_env.trigger();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let env = self.amp_env.next();
        if env <= 0.0 {
            self.active = false;
            return 0.0;
        }

        // FM shimmer: modulate filter cutoff slightly
        self.fm_phase += self.fm_increment;
        self.fm_phase -= self.fm_phase.floor();
        let fm = (self.fm_phase * 2.0 * std::f32::consts::PI).sin() * 0.15 + 1.0;
        let modulated_cutoff = self.settings.filter_freq * fm;
        self.filter.set_cutoff(modulated_cutoff.max(1000.0), self.sample_rate);

        let noise = self.noise.next();
        let filtered = self.filter.process(noise);

        filtered * env * self.settings.volume
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.update_derived_params();
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
        special_params::CYMBAL_ALGOS
    }

    fn special_params(&self) -> &'static [SpecialParamDef] {
        special_params::CYMBAL_SPECIALS
    }
}
