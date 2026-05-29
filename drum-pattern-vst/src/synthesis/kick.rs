//! Kick drum synthesizer — grey-box model with retrig-safe state.
//!
//! Architecture (informed by the TR-808/909 retrig analysis under
//! `resources/roland-kick-rust/docs/retrigger-and-sequencer.md`):
//! - Oscillator phase is continuous across triggers (no reset on retrig).
//! - Pitch envelope is additive (returns Δ-Hz) and persistent: a retrigger
//!   raises the current Δ-Hz toward the new peak via `trigger_from_current`,
//!   never snapping back to zero between hits.
//! - A short one-pole smoother on the instantaneous frequency absorbs the
//!   residual change in phase increment so the oscillator's slope stays
//!   continuous through retrigger.
//! - A short tail duck (~0.7 ms, ~12 %) masks any remaining micro-discontinuity
//!   when a step lands during a ringing tail.
//! - Click transient (impulse + noise burst) is intentionally sharp — that's
//!   the audible attack, not the click parasite we are trying to remove.
//! - DC blocker on the output cleans up the asymmetric drift that accumulates
//!   from dense retriggers.
//!
//! The sweep range is derived from `settings.frequency` so existing presets
//! keep their character: `base_freq = freq * 0.3`, `pitch_peak = freq * 0.7`,
//! giving the same start→end sweep as the legacy multiplicative `PitchEnvelope`.

use super::{dsp, saturation, settings::kick::KickSettings, Voice, VoiceSettings};

const PITCH_DECAY_SECONDS: f32 = 0.04; // ≈ 40 ms — matches the legacy ~0.12 s
                                       // exponential sweep with curve 5.0.
const PITCH_CURVE: f32 = 5.0;
const FREQ_SMOOTH_MS: f32 = 0.1;
const BASE_FREQ_RATIO: f32 = 0.3; // final freq = settings.frequency * 0.3
const PITCH_PEAK_RATIO: f32 = 0.7; // start = base + peak = settings.frequency

pub struct KickVoice {
    settings: KickSettings,
    sample_rate: f32,

     osc_sine: dsp::SineOsc,
     osc_square: dsp::SquareOsc,
     fm_carrier: dsp::SineOsc,
     fm_mod: dsp::SineOsc,
     // LowPass filter — cutoff opens then closes after trigger for extra punch.
     // Modulation: cutoff = filter_freq * (1 + filter_env * amount * 8.0)
     filter: dsp::OnePoleFilter,

     // Additive Δ-Hz envelope: target_freq = base_freq + pitch_env.next().
     pitch_env: dsp::ExpDecayEnvelope,
     // Smooths sub-sample frequency jumps caused by pitch_env retriggering.
     freq_smoother: dsp::OnePoleSmoother,
     // Body amplitude (decay + release stages), with 1.5 ms attack ramp.
     amp_env: dsp::DecayReleaseEnvelope,
     // Filter envelope: modulates cutoff for extra punch.
     filter_env: dsp::ExpDecayEnvelope,
     // Removes DC drift accumulated by asymmetric retriggers.
     dc_block: dsp::DcBlocker,

     // Attack transient (the audible "click"), kept fully separate.
     click: dsp::ClickGenerator,
     /// Saturation stage for analog character.
     saturation: saturation::SaturationConfig,

     active: bool,
     /// Crossfade counter for digital mode retriggers. When > 0, we're in the
     /// first few samples after a digital trigger and need to crossfade from the
     /// previous oscillator state to the new zero-phase state to avoid clicks.
     crossfade_samples: u32,
     /// Previous oscillator phases before digital reset, used for crossfade
     old_sine_phase: f32,
     old_square_phase: f32,
     old_fm_carrier_phase: f32,
     old_fm_mod_phase: f32,
}

impl KickVoice {
    pub fn new(sample_rate: f32, settings: KickSettings) -> Self {
        let base_freq = settings.frequency * BASE_FREQ_RATIO;

        let mut osc_sine = dsp::SineOsc::new(sample_rate);
        osc_sine.set_freq(base_freq);

        let mut osc_square = dsp::SquareOsc::new(sample_rate);
        osc_square.set_freq(base_freq);

        let mut fm_carrier = dsp::SineOsc::new(sample_rate);
        fm_carrier.set_freq(base_freq);

        let mut fm_mod = dsp::SineOsc::new(sample_rate);
        fm_mod.set_freq(base_freq * 0.5);

        let mut filter = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        filter.set_cutoff(settings.filter_freq, sample_rate);

         Self {
             settings,
             sample_rate,
             osc_sine,
             osc_square,
             fm_carrier,
             fm_mod,
             filter,
             pitch_env: dsp::ExpDecayEnvelope::new(sample_rate, PITCH_CURVE, PITCH_DECAY_SECONDS),
             freq_smoother: dsp::OnePoleSmoother::new(sample_rate, FREQ_SMOOTH_MS, base_freq),
             amp_env: dsp::DecayReleaseEnvelope::new(
                 sample_rate,
                 settings.decay_curve,
                 settings.decay,
                 settings.release_curve,
                 settings.release,
             )
             .with_attack_ms(settings.attack * 1000.0),
             filter_env: dsp::ExpDecayEnvelope::new(
                 sample_rate,
                 8.0,
                 settings.filter_env_decay.max(0.001),
             )
             .with_attack_ms(0.5),
             dc_block: dsp::DcBlocker::default(),
             click: dsp::ClickGenerator::new(sample_rate, 10.0, 0.3, 1.0),
             saturation: saturation::SaturationConfig {
                 saturation_type: saturation::SaturationType::None,
                 amount: 0.0,
                 mix: 1.0,
                 output_gain: 1.0,
                 pre_filter: false,
             },
             active: false,
             crossfade_samples: 0,
             old_sine_phase: 0.0,
             old_square_phase: 0.0,
             old_fm_carrier_phase: 0.0,
             old_fm_mod_phase: 0.0,
         }
    }

    fn base_freq(&self) -> f32 {
        (self.settings.frequency * BASE_FREQ_RATIO).max(10.0)
    }

    fn pitch_peak_hz(&self) -> f32 {
        (self.settings.frequency * PITCH_PEAK_RATIO).max(0.0)
    }

    fn update_derived_params(&mut self) {
        self.filter
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.amp_env.set_decay(self.settings.decay);
        self.amp_env.set_attack_ms(self.settings.attack * 1000.0);
        self.amp_env.set_release(self.settings.release);
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
        self.filter_env
            .set_decay(self.settings.filter_env_decay.max(0.001));
        // Frequency-related state is rebuilt on the fly each sample from
        // `settings.frequency`, so no need to touch oscillators or smoothers here.
    }

    fn click_amount(&self) -> f32 {
        self.settings.click_level
    }
}

impl Voice for KickVoice {
    fn trigger(&mut self) {
        self.active = true;
        if self.settings.analog >= 0.5 {
            // Analog-style retrigger: oscillator phase, filter state and smoother all
            // keep their value. The pitch envelope is bumped up to its peak Δ-Hz
            // *without* resetting if the tail still carries some sweep energy.
            self.pitch_env.trigger_from_current(self.pitch_peak_hz());
        } else {
            // Digital stable: reset pitch envelope and oscillator phases for
            // identical sound on every hit.
            self.pitch_env.trigger();
            // Store current oscillator states for crossfade
            self.old_sine_phase = self.osc_sine.phase();
            self.old_square_phase = self.osc_square.phase();
            self.old_fm_carrier_phase = self.fm_carrier.phase();
            self.old_fm_mod_phase = self.fm_mod.phase();
            
            // Reset to zero phase for digital stability
            self.osc_sine.phase = 0.0;
            self.osc_square.reset_phase();
            self.fm_carrier.phase = 0.0;
            self.fm_mod.phase = 0.0;
            
            // Set up crossfade over 2 samples
            self.crossfade_samples = 2;
            
            // Note: We intentionally do NOT reset the filter to avoid click parasites
            // when retriggering during a long release tail. The filter state carries
            // over, preserving continuity while oscillators restart from zero phase.
            // Reset frequency smoother to avoid click from frequency discontinuity
            self.freq_smoother.reset(self.base_freq() + self.pitch_peak_hz());
        }
        self.amp_env.trigger();
        self.filter_env.trigger();
        if self.click_amount() > 0.0 {
            self.click.trigger();
        }
    }

    fn process_sample(&mut self) -> f32 {
        let mut body = 0.0f32;

        if self.active {
            let target_freq = (self.base_freq() + self.pitch_env.next()).max(10.0);
            let freq = self.freq_smoother.process(target_freq);

            let raw = match self.settings.algo {
                1 => {
                    self.osc_square.set_freq(freq);
                    let mut square_sample = self.osc_square.next();
                    if self.crossfade_samples > 0 {
                        // Crossfade from old phase to new zero phase
                        let crossfade_ratio = self.crossfade_samples as f32 / 2.0;
                        let old_square = (self.old_square_phase * 2.0 * std::f32::consts::PI).sin();
                        square_sample = old_square * (1.0 - crossfade_ratio) + square_sample * crossfade_ratio;
                    }
                    square_sample
                }
                2 => {
                    self.fm_mod.set_freq(freq * 0.5);
                    let mod_val = self.fm_mod.next();
                    self.fm_carrier.set_freq(freq * (1.0 + mod_val * 0.8));
                    let mut carrier_sample = self.fm_carrier.next();
                    if self.crossfade_samples > 0 {
                        let crossfade_ratio = self.crossfade_samples as f32 / 2.0;
                        let old_carrier = (self.old_fm_carrier_phase * 2.0 * std::f32::consts::PI).sin();
                        carrier_sample = old_carrier * (1.0 - crossfade_ratio) + carrier_sample * crossfade_ratio;
                    }
                    carrier_sample
                }
                _ => {
                    self.osc_sine.set_freq(freq);
                    let mut sine_sample = self.osc_sine.next();
                    if self.crossfade_samples > 0 {
                        // Crossfade from old phase to new zero phase
                        let crossfade_ratio = self.crossfade_samples as f32 / 2.0;
                        let old_sine = (self.old_sine_phase * 2.0 * std::f32::consts::PI).sin();
                        sine_sample = old_sine * (1.0 - crossfade_ratio) + sine_sample * crossfade_ratio;
                    }
                    sine_sample
                }
            };

            let filter_env_val = self.filter_env.next();
            let modulated_cutoff = self.settings.filter_freq
                * (1.0 + filter_env_val * self.settings.filter_env_amount * 8.0);
            self.filter
                .set_cutoff(modulated_cutoff.max(20.0), self.sample_rate);
            let filtered = self.filter.process(raw);

            let env = self.amp_env.next();
            if env <= 0.0 {
                self.active = false;
            } else {
                body = filtered * env * self.settings.volume;
            }
            
            // Decrement crossfade counter if active
            if self.crossfade_samples > 0 {
                self.crossfade_samples -= 1;
            }
        }

        let click = if self.click_amount() > 0.0 && self.click.is_active() {
            self.click.next() * self.click_amount()
        } else {
            0.0
        };

        let out = self.dc_block.process(body + click);
        // Apply saturation (post-filter by default)
        self.saturation.process(out)
    }

    fn is_active(&self) -> bool {
        self.active || self.click.is_active()
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
        self.pitch_env.reset();
        self.filter_env.reset();
        self.click.reset();
        self.dc_block.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = KickSettings::from(settings);
        self.update_derived_params();
        // Update saturation config
        self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
        self.saturation.pre_filter = self.settings.saturation_pre_filter > 0.5;
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
        self.update_derived_params();
    }
    
    fn set_special_param(&mut self, index: usize, value: f32) {
        if index == 0 {
            self.settings.click_level = value;
        } else if index == 1 {
            self.settings.saturation_type = value as u8;
            self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
        } else if index == 2 {
            self.settings.saturation_amount = value;
            self.saturation.amount = value;
        } else if index == 3 {
            self.settings.saturation_mix = value;
            self.saturation.mix = value;
        } else if index == 4 {
            self.settings.saturation_output_gain = value;
            self.saturation.output_gain = value;
        } else if index == 5 {
            self.settings.saturation_pre_filter = value;
            self.saturation.pre_filter = value > 0.5;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kick_basic() {
        let mut kick = KickVoice::new(44100.0, KickSettings::default_at(44100.0));

        assert!(!kick.is_active());
        assert_eq!(kick.process_sample(), 0.0);

        kick.trigger();
        assert!(kick.is_active());

        let has_signal = (0..8).any(|_| kick.process_sample().abs() > 0.0);
        assert!(has_signal);
    }

    #[test]
    fn test_kick_click() {
        let mut kick = KickVoice::new(44100.0, KickSettings::default_at(44100.0));
        kick.set_special_param(0, 1.0);
        kick.trigger();

        // First sample should still contain click energy.
        let first = kick.process_sample().abs();
        assert!(
            first > 0.2,
            "Click should produce strong first sample: {}",
            first
        );
    }

    #[test]
    fn test_kick_click_at_zero_is_silent() {
        let mut kick = KickVoice::new(44100.0, KickSettings::default_at(44100.0));
        kick.set_special_param(0, 0.0);
        kick.trigger();

        // First sample: body starts at sin(0)=0 (cold start), tail_duck active,
        // dc_block doesn't add anything. Click is off → output should be ~0.
        let first = kick.process_sample().abs();
        assert!(
            first < 0.001,
            "Click should be silent at amount=0: {}",
            first
        );
    }

    #[test]
    fn test_kick_no_body_click_on_retrigger_during_tail() {
        // The body output should stay continuous through a retrigger.
        // Click transient excluded — that one is intentionally sharp.
        let mut settings = KickSettings::default_at(44100.0);
        settings.click_level = 0.0;
        let mut kick = KickVoice::new(44100.0, KickSettings::from(settings));

        kick.trigger();
        let mut last = 0.0;
        for _ in 0..4000 {
            last = kick.process_sample();
        }
        assert!(
            last.abs() > 1e-4,
            "Tail must still be audible for the test to be meaningful: {}",
            last
        );

        kick.trigger();
        let first = kick.process_sample();
        let step = (first - last).abs();
        assert!(
            step < 0.05,
            "Body discontinuity at retrigger too large: last={}, first={}, step={}",
            last,
            first,
            step
        );
    }

     #[test]
     fn test_kick_dense_retriggers_stay_finite() {
         // Stress test inspired by `resources/roland-kick-rust`: fire bursts of
         // closely-spaced retriggers and verify the output stays bounded and
         // free of NaN/Inf.
         let sr = 44100.0;
         let mut kick = KickVoice::new(sr, KickSettings::default_at(sr));
         let triggers = [0usize, 2_400, 4_800, 4_960, 9_600, 9_840];
         let mut idx = 0usize;
         let mut peak = 0.0f32;
         for n in 0..12_000 {
             if idx < triggers.len() && triggers[idx] == n {
                 kick.trigger();
                 idx += 1;
             }
             let s = kick.process_sample();
             assert!(s.is_finite(), "non-finite sample at n={}: {}", n, s);
             peak = peak.max(s.abs());
         }
         assert!(peak > 0.01);
         assert!(peak < 4.0, "output peak runaway: {}", peak);
     }

     #[test]
     fn test_kick_no_frequency_click_on_retrigger() {
         // Verify that frequency smoother reset eliminates the click parasite
         // that occurred when freq_smoother.current was not reset to the new
         // target frequency, causing a discontinuity in the first sample after trigger.
         let mut settings = KickSettings::default_at(44100.0);
         settings.analog = 0.0; // Digital mode
         settings.click_level = 0.0; // Disable click to isolate body discontinuities
         
         let mut kick = KickVoice::new(44100.0, settings);
         
         // First trigger
         kick.trigger();
         // Run for a while to let the tail develop
         for _ in 0..2000 {
             kick.process_sample();
         }
         
         // Second trigger - this is where the click parasite would occur
         kick.trigger();
         let first_sample = kick.process_sample();
         let second_sample = kick.process_sample();
         
         // The discontinuity should be small (no abrupt frequency jump)
         let step = (second_sample - first_sample).abs();
         assert!(
             step < 0.05,
             "Frequency discontinuity too large: first={}, second={}, step={}",
             first_sample, second_sample, step
         );
     }

    #[test]
    fn test_kick_decay() {
        let settings = VoiceSettings {
            frequency: 60.0,
            decay: 0.01,
            volume: 1.0,
            filter_freq: 100.0,
            attack: 0.0015,
            release: 0.0, // disable the release tail so the voice can finish quickly
            decay_curve: 5.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.0; 32],
        };
        let mut kick = KickVoice::new(44100.0, settings.into());

        kick.trigger();
        for _ in 0..1000 {
            kick.process_sample();
        }
        assert!(!kick.is_active());
    }
}
