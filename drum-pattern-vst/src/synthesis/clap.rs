//! Hand clap synthesizer.
//!
//! Architecture: layered noise bursts to emulate the "slap" reverb tail of a clap.
//! - 4 short noise bursts fired in rapid succession (~3-7 ms apart)
//! - Bandpass-ish character via highpass + gentle lowpass
//! - Exponential decay envelope

use super::{dsp, special_params, AlgoDef, SpecialParamDef, Voice, VoiceSettings};

pub struct ClapVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    noise: dsp::WhiteNoise,
    filter_hp: dsp::OnePoleFilter,
    filter_lp: dsp::OnePoleFilter,
    amp_env: dsp::ExpDecayEnvelope,

    burst_count: usize,
    burst_interval_samples: usize,
    samples_since_trigger: usize,
    active: bool,
}

impl ClapVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let mut filter_hp = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter_hp.set_cutoff(settings.filter_freq.max(800.0), sample_rate);

        let mut filter_lp = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        filter_lp.set_cutoff(6000.0, sample_rate);

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(0xBADC0FFE),
            filter_hp,
            filter_lp,
            amp_env: dsp::ExpDecayEnvelope::new(sample_rate, 6.0, settings.decay),
            burst_count: 0,
            burst_interval_samples: (0.004 * sample_rate) as usize, // 4 ms between slaps
            samples_since_trigger: 0,
            active: false,
        }
    }

    fn update_derived_params(&mut self) {
        self.filter_hp.set_cutoff(self.settings.filter_freq.max(800.0), self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
    }
}

impl Voice for ClapVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.burst_count = 0;
        self.samples_since_trigger = 0;
        self.amp_env.trigger();
        self.filter_hp.reset();
        self.filter_lp.reset();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        self.samples_since_trigger += 1;

        // Fire up to 4 bursts
        let mut burst_intensity = 1.0f32;
        if self.burst_count < 4
            && self.samples_since_trigger >= self.burst_interval_samples * self.burst_count
        {
            self.amp_env.trigger();
            self.burst_count += 1;
        }
        burst_intensity = 1.0 - ((self.burst_count.saturating_sub(1)) as f32 * 0.15);

        let env = self.amp_env.next();
        if env <= 0.0 && self.burst_count >= 4 {
            self.active = false;
            return 0.0;
        }

        let noise = self.noise.next();
        let hp = self.filter_hp.process(noise);
        let lp = self.filter_lp.process(hp);

        lp * env * burst_intensity * self.settings.volume
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
        special_params::CLAP_ALGOS
    }

    fn special_params(&self) -> &'static [SpecialParamDef] {
        special_params::CLAP_SPECIALS
    }
}
