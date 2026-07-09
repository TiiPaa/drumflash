//! Open hi-hat synthesizer.
//!
//! Similar to the closed hi-hat but with a longer decay and a brighter tail.
//! - Selectable noise source (white / pink / brown / blue)
//! - Peaking filter controlled by `settings.frequency` (labelled "Tone")
//! - Highpass filter controlled by `settings.filter_freq` (labelled "Cutoff")
//! - Shimmer path: parallel high-frequency noise layer

use super::{dsp, saturation, settings::open_hihat::OpenHiHatSettings, Voice, VoiceSettings};

/// Fixed cutoff for the shimmer highpass layer.
const SHIMMER_CUTOFF_HZ: f32 = 8000.0;
/// Fixed gain for the shimmer path so the slider is clearly audible at 1.0.
const SHIMMER_GAIN: f32 = 2.0;

pub struct OpenHiHatVoice {
    settings: OpenHiHatSettings,
    sample_rate: f32,
    // Main noise source (stereo pair) — colour selectable.
    noise: dsp::NoiseSource,
    noise_r: dsp::NoiseSource,
    // Peaking filters (stereo) — pitch the noise by boosting a narrow band.
    peaking: dsp::Biquad,
    peaking_r: dsp::Biquad,
    filter: dsp::OnePoleFilter,
    filter_r: dsp::OnePoleFilter,
    // Shimmer path (stereo pair) — parallel high-frequency air.
    shimmer_noise: dsp::BlueNoise,
    shimmer_noise_r: dsp::BlueNoise,
    shimmer_filter: dsp::OnePoleFilter,
    shimmer_filter_r: dsp::OnePoleFilter,
    envelope: dsp::DecayReleaseEnvelope,
    saturation: saturation::SaturationConfig,
    // Per-hit drift for tone, level, and timing.
    analog_drift: dsp::ToneDrift,
    /// Samples remaining in the timing delay (applied before the first sample).
    timing_delay_samples: usize,
    active: bool,
    samples_elapsed: usize,
}

impl OpenHiHatVoice {
    pub fn new(sample_rate: f32, settings: OpenHiHatSettings) -> Self {
        let mut peaking = dsp::Biquad::new();
        peaking.set_peaking(settings.frequency, settings.resonance, 6.0, sample_rate);
        let mut peaking_r = dsp::Biquad::new();
        peaking_r.set_peaking(settings.frequency, settings.resonance, 6.0, sample_rate);

        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);
        let mut filter_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter_r.set_cutoff(settings.filter_freq, sample_rate);

        let mut shimmer_filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        shimmer_filter.set_cutoff(SHIMMER_CUTOFF_HZ, sample_rate);
        let mut shimmer_filter_r = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        shimmer_filter_r.set_cutoff(SHIMMER_CUTOFF_HZ, sample_rate);

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
            noise: dsp::NoiseSource::new(settings.noise_type, 24680),
            noise_r: dsp::NoiseSource::new(settings.noise_type, 13579),
            peaking,
            peaking_r,
            filter,
            filter_r,
            shimmer_noise: dsp::BlueNoise::new(33333),
            shimmer_noise_r: dsp::BlueNoise::new(44444),
            shimmer_filter,
            shimmer_filter_r,
            envelope,
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::from(settings.saturation_type),
                amount: settings.saturation_amount,
                mix: settings.saturation_mix,
                output_gain: settings.saturation_output_gain,
                pre_filter: settings.saturation_pre_filter > 0.5,
            },
            analog_drift: dsp::ToneDrift::new(0xDEAD_C0DE, 0.25),
            timing_delay_samples: 0,
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
        self.analog_drift.trigger(self.settings.analog);
        self.timing_delay_samples = 0;
        // Désactiver la dérive de tone quand analog = 0 pour isoler level/timing.
        let drifted_freq = if self.settings.analog == 0.0 {
            self.settings.frequency
        } else {
            (self.settings.frequency * self.analog_drift.multiplier).clamp(100.0, 20000.0)
        };
        self.peaking
            .set_peaking(drifted_freq, self.settings.resonance, 6.0, self.sample_rate);
        self.peaking_r
            .set_peaking(drifted_freq, self.settings.resonance, 6.0, self.sample_rate);
        let drifted_cutoff = if self.settings.analog == 0.0 {
            self.settings.filter_freq
        } else {
            (self.settings.filter_freq * self.analog_drift.multiplier).max(100.0)
        };
        self.filter.set_cutoff(drifted_cutoff, self.sample_rate);
        self.filter_r.set_cutoff(drifted_cutoff, self.sample_rate);
        // Apply timing drift (±2 ms) to the amplitude envelope attack.
        let attack_drift_ms = (self.analog_drift.timing_offset * 2.0).clamp(-0.002, 0.002) * 1000.0;
        self.envelope
            .with_attack_ms(self.settings.attack * 1000.0 + attack_drift_ms);
        self.envelope.trigger();
    }

    fn trigger_hard(&mut self) {
        self.active = true;
        self.envelope.trigger_hard();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        // Appliquer le délai de timing (ne rien produire pendant N samples).
        if self.timing_delay_samples > 0 {
            self.timing_delay_samples -= 1;
            return 0.0;
        }

        let noise = self.noise.next();
        let peaked = self.peaking.process(noise);
        let filtered = self.filter.process(peaked);
        let shimmer = self.shimmer_filter.process(self.shimmer_noise.next())
            * self.settings.shimmer
            * SHIMMER_GAIN;
        let body = filtered + shimmer;
        let env = self.envelope.next().max(0.01);

        let time = self.samples_elapsed as f32 / self.sample_rate;
        let output = self.saturation.process(body)
            * env
            * self.settings.volume
            * self.analog_drift.level_multiplier;
        self.samples_elapsed += 1;

        if env <= 0.01 && time >= self.settings.decay {
            self.active = false;
            return 0.0;
        }

        output
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
        let shimmer_l = self.shimmer_filter.process(self.shimmer_noise.next())
            * self.settings.shimmer
            * SHIMMER_GAIN;
        let shimmer_r = self.shimmer_filter_r.process(self.shimmer_noise_r.next())
            * self.settings.shimmer
            * SHIMMER_GAIN;
        let body_l = filtered_l + shimmer_l;
        let body_r = filtered_r + shimmer_r;
        let env = self.envelope.next().max(0.01);

        let time = self.samples_elapsed as f32 / self.sample_rate;
        let vol = env * self.settings.volume * self.analog_drift.level_multiplier;
        self.samples_elapsed += 1;

        if env <= 0.01 && time >= self.settings.decay {
            self.active = false;
            return (0.0, 0.0);
        }

        (
            self.saturation.process(body_l) * vol,
            self.saturation.process(body_r) * vol,
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
        self.filter_r.reset();
        self.shimmer_filter.reset();
        self.shimmer_filter_r.reset();
        self.envelope.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        let new = OpenHiHatSettings::from(settings);
        let freq_changed = (new.frequency - self.settings.frequency).abs() > 1e-3;
        let resonance_changed = (new.resonance - self.settings.resonance).abs() > 1e-3;
        let noise_type_changed = new.noise_type != self.settings.noise_type;
        self.settings = new;

        // Only recompute the peaking biquad when the frequency or resonance
        // actually changes — avoids needless coefficient churn (and its
        // transient) on every per-step settings refresh when unchanged.
        if freq_changed || resonance_changed {
            self.peaking.set_peaking(
                self.settings.frequency,
                self.settings.resonance,
                6.0,
                self.sample_rate,
            );
            self.peaking_r.set_peaking(
                self.settings.frequency,
                self.settings.resonance,
                6.0,
                self.sample_rate,
            );
        }

        self.filter
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.filter_r
            .set_cutoff(self.settings.filter_freq, self.sample_rate);

        if noise_type_changed {
            self.noise = dsp::NoiseSource::new(self.settings.noise_type, 24680);
            self.noise_r = dsp::NoiseSource::new(self.settings.noise_type, 13579);
        }

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

    fn set_algo(&mut self, _algo: u8) {
        // Algorithme retiré de l’UI ; toujours Standard.
        self.settings.algo = 0;
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
            5 => {
                self.settings.noise_type = value as u8;
                self.noise = dsp::NoiseSource::new(self.settings.noise_type, 24680);
                self.noise_r = dsp::NoiseSource::new(self.settings.noise_type, 13579);
            }
            6 => {
                self.settings.resonance = value;
                let drifted_freq =
                    (self.settings.frequency * self.analog_drift.multiplier).clamp(100.0, 20000.0);
                self.peaking.set_peaking(
                    drifted_freq,
                    self.settings.resonance,
                    6.0,
                    self.sample_rate,
                );
                self.peaking_r.set_peaking(
                    drifted_freq,
                    self.settings.resonance,
                    6.0,
                    self.sample_rate,
                );
            }
            7 => {
                self.settings.shimmer = value;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_hihat_analog_affects_tone() {
        let sample_rate = 44100.0;
        let mut settings = VoiceSettings::open_hihat();
        settings.stereo = 0.0;
        settings.special[7] = 0.0; // disable shimmer
        let mut voice = OpenHiHatVoice::new(sample_rate, OpenHiHatSettings::from(settings));

        settings.analog = 1.0;
        voice.set_settings(settings);

        voice.trigger();
        let mut hit1 = Vec::with_capacity(2000);
        for _ in 0..2000 {
            hit1.push(voice.process_sample());
        }

        voice.trigger();
        let mut hit2 = Vec::with_capacity(2000);
        for _ in 0..2000 {
            hit2.push(voice.process_sample());
        }

        let diffs = hit1
            .iter()
            .zip(hit2.iter())
            .filter(|(a, b)| (*a - *b).abs() > 0.0001)
            .count();
        assert!(
            diffs > 200,
            "analog=1 should vary the open-hihat tone between hits (got {} diffs)",
            diffs
        );
    }
}
