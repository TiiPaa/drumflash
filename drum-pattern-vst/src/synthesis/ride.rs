//! Ride cymbal synthesizer.
//!
//! Architecture:
//! - White noise + metallic inharmonic oscillators (non-integer ratios)
//! - Highpass filter (~8 kHz) for brightness
//! - Long exponential decay with shimmer

use super::{dsp, special_params, AlgoDef, SpecialParamDef, Voice, VoiceSettings};

pub struct RideVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    noise: dsp::WhiteNoise,
    osc1: dsp::SineOsc,
    osc2: dsp::SineOsc,
    osc3: dsp::SineOsc,
    filter: dsp::OnePoleFilter,
    amp_env: dsp::DecayReleaseEnvelope,

    active: bool,
}

impl RideVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq.max(6000.0), sample_rate);

        let base_freq = settings.frequency.max(200.0);

        let mut osc1 = dsp::SineOsc::new(sample_rate);
        osc1.set_freq(base_freq * 1.0);
        let mut osc2 = dsp::SineOsc::new(sample_rate);
        osc2.set_freq(base_freq * 1.71); // inharmonic
        let mut osc3 = dsp::SineOsc::new(sample_rate);
        osc3.set_freq(base_freq * 2.41); // inharmonic

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(0xCAFE_BEEF),
            osc1,
            osc2,
            osc3,
            filter,
            amp_env: dsp::DecayReleaseEnvelope::new(
                sample_rate,
                settings.decay_curve,
                settings.decay,
                settings.release_curve,
                settings.release,
            )
            .with_attack_ms(2.0),
            active: false,
        }
    }

    fn update_derived_params(&mut self) {
        self.filter.set_cutoff(self.settings.filter_freq.max(6000.0), self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        self.amp_env.set_release(self.settings.release);
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
        let base_freq = self.settings.frequency.max(200.0);
        self.osc1.set_freq(base_freq * 1.0);
        self.osc2.set_freq(base_freq * 1.71);
        self.osc3.set_freq(base_freq * 2.41);
    }
}

impl Voice for RideVoice {
    fn trigger(&mut self) {
        self.active = true;
        // Keep oscillator phases and filter state continuous across triggers —
        // critical here because three tonal sines collapsing to phase 0
        // simultaneously is the worst case for retrigger clicks.
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

        // Metallic tone + noise shimmer
        let metallic = self.osc1.next() * 0.5
            + self.osc2.next() * 0.3
            + self.osc3.next() * 0.2;
        let noise = self.noise.next() * 0.4;
        let raw = metallic + noise;

        let filtered = self.filter.process(raw);
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
        special_params::RIDE_ALGOS
    }

    fn special_params(&self) -> &'static [SpecialParamDef] {
        special_params::RIDE_SPECIALS
    }
}
