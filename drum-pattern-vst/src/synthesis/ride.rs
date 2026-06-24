//! Ride cymbal synthesizer.
//!
//! Architecture:
//! - White noise + metallic inharmonic oscillators (non-integer ratios)
//! - Highpass filter (~8 kHz) for brightness
//! - Long exponential decay with shimmer

use super::{dsp, saturation, settings::ride::RideSettings, Voice, VoiceSettings};

pub struct RideVoice {
    settings: RideSettings,
    sample_rate: f32,

    noise: dsp::WhiteNoise,
    noise_r: dsp::WhiteNoise,
    osc1: dsp::SineOsc,
    osc2: dsp::SineOsc,
    osc3: dsp::SineOsc,
    filter: dsp::OnePoleFilter,
    filter_r: dsp::OnePoleFilter,
    amp_env: dsp::DecayReleaseEnvelope,
    saturation: saturation::SaturationConfig,

    active: bool,
}

impl RideVoice {
    pub fn new(sample_rate: f32, settings: RideSettings) -> Self {
        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq.max(6000.0), sample_rate);
        let mut filter_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter_r.set_cutoff(settings.filter_freq.max(6000.0), sample_rate);

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
            noise_r: dsp::WhiteNoise::new(0xBEEF_CAFE),
            osc1,
            osc2,
            osc3,
            filter,
            filter_r,
            amp_env: dsp::DecayReleaseEnvelope::new(
                sample_rate,
                settings.decay_curve,
                settings.decay,
                settings.release_curve,
                settings.release,
            )
            .with_attack_ms(settings.attack * 1000.0),
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::from(settings.saturation_type),
                amount: settings.saturation_amount,
                mix: settings.saturation_mix,
                output_gain: settings.saturation_output_gain,
                pre_filter: settings.saturation_pre_filter > 0.5,
            },
            active: false,
        }
    }

    fn update_derived_params(&mut self) {
        let cutoff = self.settings.filter_freq.max(6000.0);
        self.filter.set_cutoff(cutoff, self.sample_rate);
        self.filter_r.set_cutoff(cutoff, self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        self.amp_env.set_attack_ms(self.settings.attack * 1000.0);
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

    fn trigger_hard(&mut self) {
        self.active = true;
        self.amp_env.trigger_hard();
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

        // Direct mono processing without calling stereo version
        let metallic = self.osc1.next() * 0.5 + self.osc2.next() * 0.3 + self.osc3.next() * 0.2;
        let raw = metallic + self.noise.next() * 0.4;
        let filtered = self.filter.process(raw);
        self.saturation.process(filtered) * env * self.settings.volume
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        if self.settings.stereo < 0.5 {
            let m = self.process_sample();
            return (m, m);
        }

        let env = self.amp_env.next();
        if env <= 0.0 {
            self.active = false;
            return (0.0, 0.0);
        }

        let (raw_l, raw_r) = match self.settings.algo {
            1 => {
                // Bell: pure tonal, more fundamental, very little noise
                let metallic =
                    self.osc1.next() * 0.7 + self.osc2.next() * 0.15 + self.osc3.next() * 0.15;
                (
                    metallic + self.noise.next() * 0.1,
                    metallic + self.noise_r.next() * 0.1,
                )
            }
            _ => {
                // Standard: metallic + noise shimmer
                let metallic =
                    self.osc1.next() * 0.5 + self.osc2.next() * 0.3 + self.osc3.next() * 0.2;
                (
                    metallic + self.noise.next() * 0.4,
                    metallic + self.noise_r.next() * 0.4,
                )
            }
        };

        let filtered_l = self.filter.process(raw_l);
        let filtered_r = self.filter_r.process(raw_r);
        let vol = env * self.settings.volume;
        (
            self.saturation.process(filtered_l) * vol,
            self.saturation.process(filtered_r) * vol,
        )
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = RideSettings::from(settings);
        self.update_derived_params();
        self.saturation.saturation_type =
            saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
        self.saturation.pre_filter = self.settings.saturation_pre_filter > 0.5;
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        match index {
            0 => {
                self.settings.saturation_type = value as u8;
                self.saturation.saturation_type =
                    saturation::SaturationType::from(self.settings.saturation_type);
            }
            1 => {
                self.settings.saturation_amount = value;
                self.saturation.amount = value;
            }
            2 => {
                self.settings.saturation_mix = value;
                self.saturation.mix = value;
            }
            3 => {
                self.settings.saturation_output_gain = value;
                self.saturation.output_gain = value;
            }
            4 => {
                self.settings.saturation_pre_filter = value;
                self.saturation.pre_filter = value > 0.5;
            }
            _ => {}
        }
    }
}
