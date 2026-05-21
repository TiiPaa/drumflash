//! Crash cymbal synthesizer.
//!
//! Architecture:
//! - Dense white noise
//! - Highpass filter for brightness and "wash"
//! - Very long exponential decay
//! - Slight pitch modulation (FM) for the shimmering wash effect

use super::{dsp, settings::cymbal::CymbalSettings, Voice, VoiceSettings};

pub struct CymbalVoice {
    settings: CymbalSettings,
    sample_rate: f32,

    noise: dsp::WhiteNoise,
    noise_r: dsp::WhiteNoise,
    filter: dsp::OnePoleFilter,
    filter_r: dsp::OnePoleFilter,
    amp_env: dsp::DecayReleaseEnvelope,

    // FM shimmer state
    fm_phase: f32,
    fm_increment: f32,

    active: bool,
}

impl CymbalVoice {
    pub fn new(sample_rate: f32, settings: CymbalSettings) -> Self {
        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq.max(4000.0), sample_rate);
        let mut filter_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter_r.set_cutoff(settings.filter_freq.max(4000.0), sample_rate);

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(0xDEAD_BEEF),
            noise_r: dsp::WhiteNoise::new(0xCAFE_BABE),
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
            fm_phase: 0.0,
            fm_increment: 15.0 / sample_rate, // 15 Hz modulation
            active: false,
        }
    }

    fn update_derived_params(&mut self) {
        let cutoff = self.settings.filter_freq.max(4000.0);
        self.filter.set_cutoff(cutoff, self.sample_rate);
        self.filter_r.set_cutoff(cutoff, self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        self.amp_env.set_attack_ms(self.settings.attack * 1000.0);
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

        let noise = self.noise.next();
        let filtered = match self.settings.algo {
            1 => {
                // Dark: no FM shimmer, lower cutoff, darker wash
                self.filter.set_cutoff(
                    (self.settings.filter_freq * 0.6).max(1000.0),
                    self.sample_rate,
                );
                self.filter.process(noise)
            }
            _ => {
                // Standard: FM shimmer for bright wash
                self.fm_phase += self.fm_increment;
                self.fm_phase -= self.fm_phase.floor();
                let fm = (self.fm_phase * 2.0 * std::f32::consts::PI).sin() * 0.15 + 1.0;
                let modulated_cutoff = self.settings.filter_freq * fm;
                self.filter
                    .set_cutoff(modulated_cutoff.max(1000.0), self.sample_rate);
                self.filter.process(noise)
            }
        };

        filtered * env * self.settings.volume
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

        let noise_l = self.noise.next();
        let noise_r = self.noise_r.next();

        let (cutoff_l, cutoff_r) = match self.settings.algo {
            1 => {
                let c = (self.settings.filter_freq * 0.6).max(1000.0);
                (c, c)
            }
            _ => {
                self.fm_phase += self.fm_increment;
                self.fm_phase -= self.fm_phase.floor();
                let fm = (self.fm_phase * 2.0 * std::f32::consts::PI).sin() * 0.15 + 1.0;
                let c = (self.settings.filter_freq * fm).max(1000.0);
                (c, c)
            }
        };

        self.filter.set_cutoff(cutoff_l, self.sample_rate);
        self.filter_r.set_cutoff(cutoff_r, self.sample_rate);
        let filtered_l = self.filter.process(noise_l);
        let filtered_r = self.filter_r.process(noise_r);
        let vol = env * self.settings.volume;
        (filtered_l * vol, filtered_r * vol)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = CymbalSettings::from(settings);
        self.update_derived_params();
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, _index: usize, _value: f32) {
        // Cymbal has no special parameters
    }
}
