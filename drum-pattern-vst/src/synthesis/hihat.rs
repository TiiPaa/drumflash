//! Hi-Hat synthesizer
//!
//! Architecture:
//! - Selectable noise source (white / pink / brown / blue)
//! - Peaking filter (metallic "tone"; controlled by `settings.frequency`)
//! - Highpass filter (controlled by `settings.filter_freq`, labelled "Cutoff")
//! - Shimmer path: parallel high-frequency noise layer
//! - Short exponential decay (closed hi-hat)

use super::{dsp, saturation, settings::hihat::HiHatSettings, Voice, VoiceSettings};

/// Anti-click floor for the amplitude attack (a true 0 ms attack is a step = click).
const MIN_AMP_ATTACK_MS: f32 = 0.2;

/// Fixed cutoff for the shimmer highpass layer.
const SHIMMER_CUTOFF_HZ: f32 = 8000.0;
/// Fixed gain for the shimmer path so the slider is clearly audible at 1.0.
const SHIMMER_GAIN: f32 = 2.0;

/// Hi-Hat voice using filtered noise.
pub struct HiHatVoice {
    settings: HiHatSettings,
    sample_rate: f32,

    // Main noise source (stereo pair) — colour selectable.
    noise: dsp::NoiseSource,
    noise_r: dsp::NoiseSource,

    // Peaking filters (stereo pair) — pitch the noise by boosting a narrow band.
    peaking: dsp::Biquad,
    peaking_r: dsp::Biquad,

    // HighPass filters (stereo pair) — cutoff rises after trigger for bright splash then falls.
    // Modulation: cutoff = filter_freq * (1 + filter_env * amount * 1.5)
    filter: dsp::OnePoleFilter,
    filter_r: dsp::OnePoleFilter,

    // Shimmer path (stereo pair) — parallel high-frequency air.
    shimmer_noise: dsp::BlueNoise,
    shimmer_noise_r: dsp::BlueNoise,
    shimmer_filter: dsp::OnePoleFilter,
    shimmer_filter_r: dsp::OnePoleFilter,

    // Bi-stage amplitude envelope (decay + release).
    envelope: dsp::DecayReleaseEnvelope,
    // Filter envelope for splash decay.
    filter_env: dsp::ExpDecayEnvelope,
    // Saturation stage
    saturation: saturation::SaturationConfig,
    // DC blockers (per channel) — clean any offset from the tanh saturation.
    dc_block_l: dsp::DcBlocker,
    dc_block_r: dsp::DcBlocker,
    // Per-hit drift for tone, level, and timing.
    analog_drift: dsp::ToneDrift,
    /// Samples remaining in the timing delay (applied before the first sample).
    timing_delay_samples: usize,
    // Active state
    active: bool,
}

impl HiHatVoice {
    pub fn new(sample_rate: f32, settings: HiHatSettings) -> Self {
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
        .with_attack_ms((settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
        envelope.set_hold(settings.hold);

        Self {
            settings,
            sample_rate,
            noise: dsp::NoiseSource::new(settings.noise_type, 54321),
            noise_r: dsp::NoiseSource::new(settings.noise_type, 98765),
            peaking,
            peaking_r,
            filter,
            filter_r,
            shimmer_noise: dsp::BlueNoise::new(11111),
            shimmer_noise_r: dsp::BlueNoise::new(22222),
            shimmer_filter,
            shimmer_filter_r,
            envelope,
            filter_env: dsp::ExpDecayEnvelope::new(
                sample_rate,
                8.0,
                settings.filter_env_decay.max(0.001),
            )
            .with_attack_ms(0.3),
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::from(settings.saturation_type),
                amount: settings.saturation_amount,
                mix: settings.saturation_mix,
                output_gain: settings.saturation_output_gain,
                pre_filter: settings.saturation_pre_filter > 0.5,
            },
            dc_block_l: dsp::DcBlocker::default(),
            dc_block_r: dsp::DcBlocker::default(),
            analog_drift: dsp::ToneDrift::new(0xBADC0DED, 0.25),
            timing_delay_samples: 0,
            active: false,
        }
    }
}

impl Voice for HiHatVoice {
    fn trigger(&mut self) {
        self.active = true;
        // Keep noise generator and filter state continuous across triggers,
        // matching analog drum machine behaviour where the noise source is a
        // free-running zener and the filter is a passive component. The
        // envelope's attack ramp masks any residual transient.
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
        // Apply timing drift (±2 ms) to the amplitude envelope attack.
        let attack_drift_ms = (self.analog_drift.timing_offset * 2.0).clamp(-0.002, 0.002) * 1000.0;
        self.envelope.set_attack_ms(
            (self.settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS) + attack_drift_ms,
        );
        self.envelope.trigger();
        self.filter_env.trigger();
    }

    fn trigger_hard(&mut self) {
        self.active = true;
        self.envelope.trigger_hard();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        if self.timing_delay_samples > 0 {
            self.timing_delay_samples -= 1;
            return 0.0;
        }

        // Apply amplitude envelope
        let env = self.envelope.next();
        let filter_env_val = self.filter_env.next();

        let modulated_cutoff = self.settings.filter_freq
            * (1.0 + filter_env_val * self.settings.filter_env_amount * 1.5)
            * self.analog_drift.multiplier;
        self.filter
            .set_cutoff(modulated_cutoff.max(100.0), self.sample_rate);

        let noise = self.noise.next();
        let peaked = self.peaking.process(noise);
        let filtered = self.filter.process(peaked);

        // Shimmer: parallel high-frequency air.
        let shimmer = self.shimmer_filter.process(self.shimmer_noise.next())
            * self.settings.shimmer
            * SHIMMER_GAIN;
        let body = filtered + shimmer;

        let output = body * env * self.settings.volume * self.analog_drift.level_multiplier;

        // Stop when silent
        if !self.envelope.is_active() {
            self.active = false;
            return 0.0;
        }

        self.dc_block_l.process(output)
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        if self.settings.stereo < 0.5 {
            let m = self.process_sample();
            return (m, m);
        }

        let env = self.envelope.next();
        let filter_env_val = self.filter_env.next();

        let modulated_cutoff = self.settings.filter_freq
            * (1.0 + filter_env_val * self.settings.filter_env_amount * 1.5)
            * self.analog_drift.multiplier;
        let cutoff = modulated_cutoff.max(100.0);
        self.filter.set_cutoff(cutoff, self.sample_rate);
        self.filter_r.set_cutoff(cutoff, self.sample_rate);

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

        if !self.envelope.is_active() {
            self.active = false;
            return (0.0, 0.0);
        }

        let vol = env * self.settings.volume * self.analog_drift.level_multiplier;
        (
            self.dc_block_l.process(body_l * vol),
            self.dc_block_r.process(body_r * vol),
        )
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.peaking.reset();
        self.peaking_r.reset();
        self.filter.reset();
        self.filter_r.reset();
        self.shimmer_filter.reset();
        self.shimmer_filter_r.reset();
        self.envelope.reset();
        self.filter_env.reset();
        self.dc_block_l.reset();
        self.dc_block_r.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        let new = HiHatSettings::from(settings);
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
            self.noise = dsp::NoiseSource::new(self.settings.noise_type, 54321);
            self.noise_r = dsp::NoiseSource::new(self.settings.noise_type, 98765);
        }

        // Update the amp envelope via setters (preserve tail state across
        // retriggers; recreating it reset the value to 0 = a retrigger click).
        self.envelope.set_decay(self.settings.decay);
        self.envelope.set_release(self.settings.release);
        self.envelope.set_decay_curve(self.settings.decay_curve);
        self.envelope.set_release_curve(self.settings.release_curve);
        // Update attack with timing drift applied in trigger().
        self.envelope
            .set_attack_ms((self.settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
        self.envelope.set_hold(self.settings.hold);
        self.filter_env
            .set_decay(self.settings.filter_env_decay.max(0.001));
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
                self.noise = dsp::NoiseSource::new(self.settings.noise_type, 54321);
                self.noise_r = dsp::NoiseSource::new(self.settings.noise_type, 98765);
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
    fn test_hihat_basic() {
        let mut hihat = HiHatVoice::new(44100.0, HiHatSettings::from(VoiceSettings::hihat()));

        assert!(!hihat.is_active());
        assert_eq!(hihat.process_sample(), 0.0);

        hihat.trigger();
        assert!(hihat.is_active());

        let sample = hihat.process_sample();
        assert!(sample.abs() > 0.0);
    }

    #[test]
    fn test_hihat_analog_affects_tone() {
        // With analog = 0 the peaking center is fixed; with analog = 1 it drifts
        // per hit. We compare two consecutive hits on the same voice.
        let sample_rate = 44100.0;
        let mut settings = VoiceSettings::hihat();
        settings.stereo = 0.0;
        settings.special[7] = 0.0; // disable shimmer to isolate the peaking path
        let mut hihat = HiHatVoice::new(sample_rate, HiHatSettings::from(settings));

        // Hit 1 with analog = 1.
        settings.analog = 1.0;
        hihat.set_settings(settings);
        hihat.trigger();
        let mut hit1 = Vec::with_capacity(2000);
        for _ in 0..2000 {
            hit1.push(hihat.process_sample());
        }

        // Hit 2 with analog = 1 on the same voice.
        hihat.trigger();
        let mut hit2 = Vec::with_capacity(2000);
        for _ in 0..2000 {
            hit2.push(hihat.process_sample());
        }

        let diffs = hit1
            .iter()
            .zip(hit2.iter())
            .filter(|(a, b)| (*a - *b).abs() > 0.0001)
            .count();
        assert!(
            diffs > 200,
            "analog=1 should vary the hi-hat tone between hits (got {} diffs)",
            diffs
        );
    }

    #[test]
    fn test_hihat_short_decay() {
        // Hi-hat should have very short decay
        let settings = VoiceSettings {
            frequency: 8000.0,
            decay: 0.05, // 50ms
            volume: 1.0,
            filter_freq: 10000.0,
            attack: 0.0003,
            release: 0.0, // disable release so the decay test is meaningful
            decay_curve: 8.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.0; 32],
        };
        let mut hihat = HiHatVoice::new(44100.0, HiHatSettings::from(settings));

        hihat.trigger();

        // Process 100ms worth of samples
        for _ in 0..4410 {
            hihat.process_sample();
        }

        // Should be stopped
        assert!(!hihat.is_active());
    }
}
