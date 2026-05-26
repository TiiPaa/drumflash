//! Open hi-hat synthesizer.
//!
//! Similar to the closed hi-hat but with a longer decay and a brighter tail.
//! Peaking filter controlled by settings.frequency adds a pitched metallic peak.

use super::{dsp, saturation, settings::open_hihat::OpenHiHatSettings, Voice, VoiceSettings};

pub struct OpenHiHatVoice {
    settings: OpenHiHatSettings,
    sample_rate: f32,
    noise: dsp::WhiteNoise,
    noise_r: dsp::WhiteNoise,
    // Peaking filters (stereo) — pitch the noise by boosting a narrow band.
    peaking: dsp::Biquad,
    peaking_r: dsp::Biquad,
    filter: dsp::OnePoleFilter,
    filter_r: dsp::OnePoleFilter,
    envelope: dsp::DecayReleaseEnvelope,
    saturation: saturation::SaturationConfig,
    active: bool,
    samples_elapsed: usize,
}

impl OpenHiHatVoice {
    pub fn new(sample_rate: f32, settings: OpenHiHatSettings) -> Self {
        let mut peaking = dsp::Biquad::new();
        peaking.set_peaking(settings.frequency, 2.0, 6.0, sample_rate);
        let mut peaking_r = dsp::Biquad::new();
        peaking_r.set_peaking(settings.frequency, 2.0, 6.0, sample_rate);

        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);
        let mut filter_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter_r.set_cutoff(settings.filter_freq, sample_rate);

        let mut envelope = dsp::DecayReleaseEnvelope::new(
            sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms(settings.attack * 1000.0);
        envelope.set_hold(settings.hold);

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(24680),
            noise_r: dsp::WhiteNoise::new(13579),
            peaking,
            peaking_r,
            filter,
            filter_r,
            envelope,
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::None,
                amount: 0.0,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
            },
            active: false,
            samples_elapsed: 0,
        }
    }
}

impl Voice for OpenHiHatVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.samples_elapsed = 0;
        // Keep noise generator and filter state continuous across triggers.
        self.envelope.trigger();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let noise = self.noise.next();
        let peaked = self.peaking.process(noise);
        let filtered = self.filter.process(peaked);
        let env = self.envelope.next().max(0.01);

        let time = self.samples_elapsed as f32 / self.sample_rate;
        let output = filtered * env * self.settings.volume;
        self.samples_elapsed += 1;

        if env <= 0.01 && time >= self.settings.decay {
            self.active = false;
            return 0.0;
        }

        self.saturation.process(output)
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        if self.settings.stereo < 0.5 {
            let m = self.process_sample();
            return (m, m);
        }

        let noise_l = self.noise.next();
        let noise_r = self.noise_r.next();
        let peaked_l = self.peaking.process(noise_l);
        let peaked_r = self.peaking_r.process(noise_r);
        let filtered_l = self.filter.process(peaked_l);
        let filtered_r = self.filter_r.process(peaked_r);
        let env = self.envelope.next().max(0.01);

        let time = self.samples_elapsed as f32 / self.sample_rate;
        let vol = env * self.settings.volume;
        self.samples_elapsed += 1;

        if env <= 0.01 && time >= self.settings.decay {
            self.active = false;
            return (0.0, 0.0);
        }

        (
            self.saturation.process(filtered_l * vol),
            self.saturation.process(filtered_r * vol),
        )
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.samples_elapsed = 0;
        self.peaking.reset();
        self.peaking_r.reset();
        self.filter.reset();
        self.envelope.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = OpenHiHatSettings::from(settings);
        self.peaking
            .set_peaking(self.settings.frequency, 2.0, 6.0, self.sample_rate);
        self.peaking_r
            .set_peaking(self.settings.frequency, 2.0, 6.0, self.sample_rate);
        self.filter
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.filter_r
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.envelope = dsp::DecayReleaseEnvelope::new(
            self.sample_rate,
            self.settings.decay_curve,
            self.settings.decay,
            self.settings.release_curve,
            self.settings.release,
        )
        .with_attack_ms(self.settings.attack * 1000.0);
        self.envelope.set_hold(self.settings.hold);
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
        if index == 0 {
            self.settings.saturation_type = value as u8;
            self.saturation.saturation_type =
                saturation::SaturationType::from(self.settings.saturation_type);
        } else if index == 1 {
            self.settings.saturation_amount = value;
            self.saturation.amount = value;
        } else if index == 2 {
            self.settings.saturation_mix = value;
            self.saturation.mix = value;
        } else if index == 3 {
            self.settings.saturation_output_gain = value;
            self.saturation.output_gain = value;
        } else if index == 4 {
            self.settings.saturation_pre_filter = value;
            self.saturation.pre_filter = value > 0.5;
        }
    }
}
