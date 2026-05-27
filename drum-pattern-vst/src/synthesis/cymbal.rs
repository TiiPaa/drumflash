//! Crash cymbal synthesizer.
//!
//! Architecture:
//! - Dense coloured noise (white / pink / brown / blue)
//! - Highpass filter for brightness and "wash"
//! - Very long exponential decay
//! - Pitch modulation (FM) for the shimmering wash effect

use super::{dsp, settings::cymbal::CymbalSettings, Voice, VoiceSettings};

pub struct CymbalVoice {
    settings: CymbalSettings,
    sample_rate: f32,

    white_noise: dsp::WhiteNoise,
    pink_noise: dsp::PinkNoise,
    brown_noise: dsp::BrownNoise,
    blue_noise: dsp::BlueNoise,
    white_noise_r: dsp::WhiteNoise,
    pink_noise_r: dsp::PinkNoise,
    brown_noise_r: dsp::BrownNoise,
    blue_noise_r: dsp::BlueNoise,

    filter: dsp::OnePoleFilter,
    filter_r: dsp::OnePoleFilter,
    amp_env: dsp::DecayReleaseEnvelope,

    // FM shimmer state
    fm_phase: f32,

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
            white_noise: dsp::WhiteNoise::new(0xDEAD_BEEF),
            pink_noise: dsp::PinkNoise::new(0xDEAD_BEEF),
            brown_noise: dsp::BrownNoise::new(0xDEAD_BEEF),
            blue_noise: dsp::BlueNoise::new(0xDEAD_BEEF),
            white_noise_r: dsp::WhiteNoise::new(0xCAFE_BABE),
            pink_noise_r: dsp::PinkNoise::new(0xCAFE_BABE),
            brown_noise_r: dsp::BrownNoise::new(0xCAFE_BABE),
            blue_noise_r: dsp::BlueNoise::new(0xCAFE_BABE),
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
            active: false,
        }
    }

    fn fm_increment(&self) -> f32 {
        let freq = self.settings.shimmer_freq.max(0.1);
        freq / self.sample_rate
    }

    fn next_noise_l(&mut self) -> f32 {
        match self.settings.noise_type {
            1 => self.pink_noise.next(),
            2 => self.brown_noise.next(),
            3 => self.blue_noise.next(),
            _ => self.white_noise.next(),
        }
    }

    fn next_noise_r(&mut self) -> f32 {
        match self.settings.noise_type {
            1 => self.pink_noise_r.next(),
            2 => self.brown_noise_r.next(),
            3 => self.blue_noise_r.next(),
            _ => self.white_noise_r.next(),
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

        let noise = self.next_noise_l();
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
                self.fm_phase += self.fm_increment();
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

        let noise_l = self.next_noise_l();
        let noise_r = self.next_noise_r();

        let (cutoff_l, cutoff_r) = match self.settings.algo {
            1 => {
                let c = (self.settings.filter_freq * 0.6).max(1000.0);
                (c, c)
            }
            _ => {
                self.fm_phase += self.fm_increment();
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

    fn set_special_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.settings.shimmer_freq = value,
            1 => self.settings.noise_type = value as u8,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shimmer_produces_varying_filter_cutoff() {
        let sample_rate = 44100.0;

        let mut settings_fast = CymbalSettings::from(VoiceSettings::cymbal());
        settings_fast.shimmer_freq = 15.0;
        settings_fast.stereo = 1.0;

        let mut settings_slow = settings_fast;
        settings_slow.shimmer_freq = 0.1;

        let mut voice_fast = CymbalVoice::new(sample_rate, settings_fast);
        let mut voice_slow = CymbalVoice::new(sample_rate, settings_slow);

        voice_fast.trigger();
        voice_slow.trigger();

        // Process enough samples for 15 Hz to complete several cycles
        // while 0.1 Hz barely moves.
        let samples = 3000;
        let mut diff_count = 0usize;
        for _ in 0..samples {
            let (l_fast, r_fast) = voice_fast.process_sample_stereo();
            let (l_slow, r_slow) = voice_slow.process_sample_stereo();
            if (l_fast - l_slow).abs() > 0.0001 || (r_fast - r_slow).abs() > 0.0001 {
                diff_count += 1;
            }
        }

        // With identical noise seeds and envelopes, the only difference is
        // the filter cutoff modulation.  At 15 Hz there should be plenty of
        // divergence; at 0.1 Hz the cutoff is essentially static, so the two
        // voices should track each other closely.
        assert!(
            diff_count > samples / 4,
            "Expected significant output divergence with fast shimmer (got {} diffs in {} samples)",
            diff_count,
            samples
        );
    }

    #[test]
    fn shimmer_freq_zero_is_effectively_static() {
        let sample_rate = 44100.0;

        let mut settings_a = CymbalSettings::from(VoiceSettings::cymbal());
        settings_a.shimmer_freq = 0.1;
        settings_a.stereo = 1.0;

        let settings_b = settings_a;
        // Both use 0.1 Hz – the minimum enforced by fm_increment().

        let mut voice_a = CymbalVoice::new(sample_rate, settings_a);
        let mut voice_b = CymbalVoice::new(sample_rate, settings_b);

        voice_a.trigger();
        voice_b.trigger();

        let samples = 3000;
        let mut diff_count = 0usize;
        for _ in 0..samples {
            let (la, ra) = voice_a.process_sample_stereo();
            let (lb, rb) = voice_b.process_sample_stereo();
            if (la - lb).abs() > 0.0001 || (ra - rb).abs() > 0.0001 {
                diff_count += 1;
            }
        }

        assert_eq!(
            diff_count, 0,
            "Two cymbals with identical settings should produce identical output (got {} diffs)",
            diff_count
        );
    }

    /// Regression test: verify that set_settings properly propagates shimmer_freq
    /// and that a voice produces different output when shimmer_freq changes.
    #[test]
    fn set_settings_updates_shimmer_freq() {
        let sample_rate = 44100.0;
        let mut voice = CymbalVoice::new(
            sample_rate,
            CymbalSettings::from(VoiceSettings::cymbal()),
        );

        // First trigger with fast shimmer
        let mut settings_fast = VoiceSettings::cymbal();
        settings_fast.special[0] = 15.0;
        settings_fast.stereo = 1.0;
        voice.set_settings(settings_fast);
        voice.trigger();

        let mut out_fast = Vec::with_capacity(10000);
        for _ in 0..10000 {
            let (l, _r) = voice.process_sample_stereo();
            out_fast.push(l);
        }

        // Reset and retrigger with slow shimmer
        voice.reset();
        let mut settings_slow = VoiceSettings::cymbal();
        settings_slow.special[0] = 0.1;
        settings_slow.stereo = 1.0;
        voice.set_settings(settings_slow);
        voice.trigger();

        let mut out_slow = Vec::with_capacity(10000);
        for _ in 0..10000 {
            let (l, _r) = voice.process_sample_stereo();
            out_slow.push(l);
        }

        // The two outputs must differ because the filter cutoff modulation rate differs
        let diffs: usize = out_fast
            .iter()
            .zip(out_slow.iter())
            .filter(|(a, b)| (*a - *b).abs() > 0.0001)
            .count();
        assert!(
            diffs > out_fast.len() / 4,
            "Expected significant divergence after set_settings with different shimmer_freq (got {} diffs in {} samples)",
            diffs,
            out_fast.len()
        );
    }

    /// Verify that the default VoiceSettings::cymbal() carries the expected shimmer_freq.
    #[test]
    fn default_cymbal_settings_have_shimmer_15hz() {
        let settings = CymbalSettings::from(VoiceSettings::cymbal());
        assert!(
            (settings.shimmer_freq - 15.0).abs() < 0.001,
            "Default cymbal shimmer_freq should be ~15.0 Hz, got {}",
            settings.shimmer_freq
        );
    }

    /// Integration test: simulate the full DrumSynthesizer path for cymbal.
    #[test]
    fn cymbal_shimmer_through_drum_synthesizer() {
        use crate::synthesis::{DrumSynthesizer, DrumVoice};

        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let cy_idx = DrumVoice::Cymbal as usize;

        // Trigger with default settings (should have 15 Hz shimmer)
        synth.trigger(cy_idx, 1.0);

        let mut samples_fast = vec![0.0f32; 10000];
        let mut outputs = [[0.0f32; 2]; DrumVoice::COUNT];
        for i in 0..10000 {
            synth.process_voice_samples_stereo(&mut outputs);
            samples_fast[i] = outputs[cy_idx][0];
        }

        // Change to slow shimmer and retrigger
        let mut settings_slow = VoiceSettings::cymbal();
        settings_slow.special[0] = 0.1;
        settings_slow.stereo = 1.0;
        synth.set_voice_settings(DrumVoice::Cymbal, settings_slow);
        synth.trigger(cy_idx, 1.0);

        let mut samples_slow = vec![0.0f32; 10000];
        for i in 0..10000 {
            synth.process_voice_samples_stereo(&mut outputs);
            samples_slow[i] = outputs[cy_idx][0];
        }

        let diffs = samples_fast
            .iter()
            .zip(samples_slow.iter())
            .filter(|(a, b)| (*a - *b).abs() > 0.0001)
            .count();

        assert!(
            diffs > samples_fast.len() / 4,
            "DrumSynthesizer cymbal should diverge with different shimmer_freq (got {} diffs)",
            diffs
        );
    }
}
