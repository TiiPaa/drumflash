//! Snare drum synthesizer
//!
//! Three algorithms:
//! - 0 Synth: triangle osc + noise (original Web Audio model)
//! - 1 Noise: pure white noise
//! - 2 Layered: fundamental + 2nd harmonic + noise
//!
//! For the analog TR-606 bridged-T snare model, see the separate
//! `Snare606Voice` (voice index 10).

use super::{dsp, special_params, AlgoDef, SpecialParamDef, Voice, VoiceSettings};

/// Anti-click attack ramp (mimics analog VCA RC charge time).
const SNARE_ATTACK_MS: f32 = 1.5;

/// Snare drum voice using triangle oscillator + noise
pub struct SnareVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    // Oscillator (triangle) for body
    osc: dsp::TriangleOsc,

    // Noise generator
    noise: dsp::WhiteNoise,

    // Highpass filter
    filter: dsp::OnePoleFilter,

    // Bi-stage amplitude envelope (decay + release).
    envelope: dsp::DecayReleaseEnvelope,

    // Active state
    active: bool,
}

impl SnareVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let mut osc = dsp::TriangleOsc::new(sample_rate);
        osc.set_freq(settings.frequency);

        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);

        let mut envelope = dsp::DecayReleaseEnvelope::new(
            sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms(SNARE_ATTACK_MS);
        envelope.set_hold(settings.hold);

        Self {
            settings,
            sample_rate,
            osc,
            noise: dsp::WhiteNoise::new(12345),
            filter,
            envelope,
            active: false,
        }
    }
}

impl Voice for SnareVoice {
    fn trigger(&mut self) {
        self.active = true;
        // Keep oscillator phase, noise generator and filter state continuous
        // across triggers — see kick.rs for the rationale.
        self.envelope.trigger();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let snap = self.settings.special[0];
        let env = self.envelope.next();

        let output = match self.settings.algo {
            1 => {
                // Noise: pure white noise, no oscillator
                let mixed = self.noise.next() * 0.5;
                let filtered = self.filter.process(mixed);
                filtered * env * self.settings.volume
            }
            2 => {
                // Layered: fundamental + overtone + noise
                let fundamental = self.osc.next();
                let overtone = ((self.osc.phase * 2.0) * 2.0 * std::f32::consts::PI).sin() * 0.3;
                let osc = (fundamental + overtone) * snap * 0.5;
                let noise = self.noise.next() * (1.0 - snap) * 0.5;
                let filtered = self.filter.process(osc + noise);
                filtered * env * self.settings.volume
            }
            _ => {
                // Synth: triangle osc + noise (ratio controlled by snap)
                let osc_gain = snap * 0.5;
                let noise_gain = (1.0 - snap) * 0.5;
                let osc = self.osc.next() * osc_gain;
                let noise = self.noise.next() * noise_gain;
                let filtered = self.filter.process(osc + noise);
                filtered * env * self.settings.volume
            }
        };

        // Stop when envelope is too low
        if !self.envelope.is_active() {
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
        self.osc.reset();
        self.filter.reset();
        self.envelope.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.osc.set_freq(settings.frequency);
        self.filter.set_cutoff(settings.filter_freq, self.sample_rate);
        self.envelope = dsp::DecayReleaseEnvelope::new(
            self.sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms(SNARE_ATTACK_MS);
        self.envelope.set_hold(settings.hold);
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
        special_params::SNARE_ALGOS
    }

    fn special_params(&self) -> &'static [SpecialParamDef] {
        special_params::SNARE_SPECIALS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snare_basic() {
        let mut snare = SnareVoice::new(44100.0, VoiceSettings::snare());

        // Silent before trigger
        assert!(!snare.is_active());
        assert_eq!(snare.process_sample(), 0.0);

        // Trigger
        snare.trigger();
        assert!(snare.is_active());

        // Should produce sound
        let sample = snare.process_sample();
        assert!(sample.abs() > 0.0);
    }

    #[test]
    fn test_snare_has_noise() {
        let settings = VoiceSettings::snare();
        let mut snare = SnareVoice::new(44100.0, settings);

        snare.trigger();

        // Get multiple samples
        let samples: Vec<f32> = (0..100).map(|_| snare.process_sample()).collect();

        // Should have variation (noise component)
        let sum: f32 = samples.iter().sum();
        assert!(sum != 0.0);
    }
}
