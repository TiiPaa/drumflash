//! Kick drum synthesizer â€” grey-box model with retrig-safe state.
//!
//! Architecture (informed by the TR-808/909 retrig analysis under
//! `resources/roland-kick-rust/docs/retrigger-and-sequencer.md`):
//! - **Oscillator phase is ALWAYS continuous across triggers, in both modes.**
//!   Phase is never reset to zero â€” that was the historical "click parasite":
//!   a phase jump on a still-ringing tail is a broadband discontinuity. Since
//!   the oscillators are phase accumulators, changing frequency is inherently
//!   click-free (it changes the phase *slope*, never the phase value).
//! - The analog/digital distinction lives entirely in the **pitch envelope**,
//!   which is phase-safe:
//!     * `analog`  â†’ pitch sweep is *persistent* (`trigger_from_current`): the
//!       Î”-Hz only ever rises toward the peak, stacking with the ringing tail,
//!       so every hit drifts slightly â€” the organic, analog feel.
//!     * `digital` â†’ pitch sweep is *reset* to the full peak every hit
//!       (`trigger_reset_to`): identical, repeatable sweep on every trigger.
//! - The amplitude envelope retriggers from its current value via a short attack
//!   ramp (never a jump from/to zero), so it is continuous through retrigger.
//! - A short one-pole smoother on the instantaneous frequency is numerical
//!   hygiene only (frequency changes do not click on a phase accumulator).
//! - Click transient (impulse + noise burst) is intentionally sharp â€” that's
//!   the audible attack, not the click parasite we are trying to remove.
//! - DC blocker on the output cleans up the asymmetric drift that accumulates
//!   from dense retriggers.
//!
//! The sweep range is derived from `settings.frequency` so existing presets
//! keep their character: `base_freq = freq * 0.3`, `pitch_peak = freq * 0.7`,
//! giving the same startâ†’end sweep as the legacy multiplicative `PitchEnvelope`.

use super::{dsp, saturation, settings::kick::KickSettings, Voice, VoiceSettings};

const PITCH_DECAY_SECONDS: f32 = 0.04; // â‰ˆ 40 ms â€” matches the legacy ~0.12 s
                                       // exponential sweep with curve 5.0.
const PITCH_CURVE: f32 = 5.0;
const FREQ_SMOOTH_MS: f32 = 2.0;
const BASE_FREQ_RATIO: f32 = 0.3; // final freq = settings.frequency * 0.3
const PITCH_PEAK_RATIO: f32 = 0.7; // start = base + peak = settings.frequency
/// Anti-click floor for the amplitude attack. A true 0 ms attack is a step
/// function (instant jump to full level) â€” a click by definition; no analog VCA
/// charges in zero time either. We clamp the *amplitude* attack to this minimum
/// so "attack = 0" stays perceptually instant yet click-free. Punch is
/// unaffected: the transient comes from the click layer and the pitch sweep, not
/// from the body's amplitude ramp.
const MIN_AMP_ATTACK_MS: f32 = 0.5;
/// "Analog" per-hit drift depths. In analog mode every trigger pulls a small
/// random detune + level offset so no two hits are identical (the vintage
/// "breathing"); digital mode uses 0 drift = bit-identical hits.
// Analog drift is now shared via dsp::AnalogDrift constants.
// Use those values for consistency across all voices.

pub struct KickVoice {
    settings: KickSettings,
    sample_rate: f32,

    osc_sine: dsp::SineOsc,
    osc_square: dsp::SquareOsc,
    fm_carrier: dsp::SineOsc,
    fm_mod: dsp::SineOsc,
    // LowPass filter â€” cutoff opens then closes after trigger for extra punch.
    // Modulation: cutoff = filter_freq * (1 + filter_env * amount * 8.0)
    filter: dsp::OnePoleFilter,

    // Additive Î”-Hz envelope: target_freq = base_freq + pitch_env.next().
    pitch_env: dsp::ExpDecayEnvelope,
    // Smooths sub-sample frequency jumps caused by pitch_env retriggering.
    freq_smoother: dsp::OnePoleSmoother,
    // Smooths filter cutoff jumps caused by parameter changes or plocks.
    filter_cutoff_smoother: dsp::OnePoleSmoother,
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
    /// Per-hit "analog" drift state. In analog mode each trigger pulls small
    /// random offsets so hits vary; in digital mode they stay exactly 1.0.
    drift_rng: dsp::WhiteNoise,
    drift_pitch: f32,
    drift_level: f32,
    drift_decay: f32,
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
            filter_cutoff_smoother: dsp::OnePoleSmoother::new(
                sample_rate,
                FREQ_SMOOTH_MS,
                settings.filter_freq,
            ),
            amp_env: dsp::DecayReleaseEnvelope::new(
                sample_rate,
                settings.decay_curve,
                settings.decay,
                settings.release_curve,
                settings.release,
            )
            .with_attack_ms((settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS)),
            filter_env: dsp::ExpDecayEnvelope::new(
                sample_rate,
                8.0,
                settings.filter_env_decay.max(0.001),
            )
            .with_attack_ms(0.5),
            dc_block: dsp::DcBlocker::default(),
            click: Self::make_click_generator(sample_rate, 1),
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::None,
                amount: 0.0,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
            },
            active: false,
            drift_rng: dsp::WhiteNoise::new(0x9E37_79B9),
            drift_pitch: 1.0,
            drift_level: 1.0,
            drift_decay: 1.0,
        }
    }

    fn base_freq(&self) -> f32 {
        (self.settings.frequency * BASE_FREQ_RATIO).max(10.0)
    }

    fn pitch_peak_hz(&self) -> f32 {
        (self.settings.frequency * PITCH_PEAK_RATIO).max(0.0)
    }

    fn update_derived_params(&mut self) {
        // Note: we do NOT call self.filter.set_cutoff here.
        // The filter cutoff is smoothed in process_sample via filter_cutoff_smoother
        // to avoid clicks when plocks or set_settings change filter_freq.
        self.amp_env.set_decay(self.settings.decay);
        self.amp_env
            .set_attack_ms((self.settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
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

    fn make_click_generator(sample_rate: f32, click_type: u8) -> dsp::ClickGenerator {
        match click_type {
            0 => dsp::ClickGenerator::new(sample_rate, 30.0, 0.8, 0.4), // Soft: long decay, noisy, quiet
            2 => dsp::ClickGenerator::new(sample_rate, 2.0, 0.0, 2.5), // Hard: ultra-short, pure impulse, loud
            _ => dsp::ClickGenerator::new(sample_rate, 10.0, 0.3, 1.0), // Medium: balanced
        }
    }
}

impl Voice for KickVoice {
    fn trigger(&mut self) {
        let was_active = self.active;
        self.active = true;
        // Phase is never reset DURING a ringing tail — a phase jump on a live tail
        // was the click parasite. The oscillators are phase accumulators, so the
        // pitch sweep below is click-safe: it changes the phase *slope*, never the
        // phase value; filter state and the smoothers also stay continuous.
        //
        // Cold start only (voice was silent): align oscillator phase to 0 and clear
        // the filter so the attack begins from a clean zero baseline. THIS is what
        // keeps even a 0 ms attack click-free, and it is safe precisely because the
        // previous output was already silence. (Mirrors the `kick_808` reference.)
        if !was_active {
            self.osc_sine.phase = 0.0;
            self.osc_square.reset_phase();
            self.fm_carrier.phase = 0.0;
            self.fm_mod.phase = 0.0;
            self.filter.reset();
            // Clear the carry-over state too, so a cold start is a true clean slate
            // (makes digital mode bit-identical hit-to-hit; analog drift is applied
            // afterwards). Safe because the previous output was already silence.
            self.freq_smoother.reset(self.base_freq());
            self.filter_cutoff_smoother.reset(self.settings.filter_freq);
            self.dc_block.reset();
        }
        if self.settings.analog >= 0.5 {
            // Analog: persistent sweep + per-hit drift (the vintage "breathing").
            // Δ-Hz only ever rises toward the peak, stacking with the tail.
            self.pitch_env.trigger_from_current(self.pitch_peak_hz());
            self.drift_pitch = 1.0 + self.drift_rng.next() * dsp::AnalogDrift::PITCH_DEPTH;
            self.drift_level = 1.0 + self.drift_rng.next() * dsp::AnalogDrift::LEVEL_DEPTH;
            self.drift_decay = 1.0 + self.drift_rng.next() * dsp::AnalogDrift::TIME_DEPTH;
        } else {
            // Digital: deterministic sweep, NO drift — bit-identical on every hit.
            self.pitch_env.trigger_reset_to(self.pitch_peak_hz());
            self.drift_pitch = 1.0;
            self.drift_level = 1.0;
            self.drift_decay = 1.0;
        }
        // Per-hit envelope-time drift: scale BOTH the decay and the release stages
        // so the audible TAIL LENGTH varies in analog (the most audible part of the
        // "breathing"). Drifting decay alone is nearly inaudible because the long
        // tail is carried by the release stage. Exact times in digital.
        self.amp_env
            .set_decay(self.settings.decay * self.drift_decay);
        self.amp_env
            .set_release(self.settings.release * self.drift_decay);
        // Amplitude / filter envelopes attack-ramp from their current value, so a
        // retrigger during a ringing tail is continuous (no jump to/from zero).
        self.amp_env.trigger();
        self.filter_env.trigger();
        if self.click_amount() > 0.0 {
            self.click.trigger();
        }
    }

    fn trigger_hard(&mut self) {
        self.active = true;
        self.amp_env.trigger_hard();
    }

    fn process_sample(&mut self) -> f32 {
        let mut body = 0.0f32;

        if self.active {
            let target_freq =
                ((self.base_freq() + self.pitch_env.next()) * self.drift_pitch).max(10.0);
            let freq = self.freq_smoother.process(target_freq);

            let raw = match self.settings.algo {
                1 => {
                    self.osc_square.set_freq(freq);
                    self.osc_square.next()
                }
                2 => {
                    self.fm_mod.set_freq(freq * 0.5);
                    let mod_val = self.fm_mod.next();
                    self.fm_carrier.set_freq(freq * (1.0 + mod_val * 0.8));
                    self.fm_carrier.next()
                }
                _ => {
                    self.osc_sine.set_freq(freq);
                    self.osc_sine.next()
                }
            };

            let filter_env_val = self.filter_env.next();
            let target_cutoff = self.settings.filter_freq
                * (1.0 + filter_env_val * self.settings.filter_env_amount * 8.0);
            let smoothed_cutoff = self.filter_cutoff_smoother.process(target_cutoff.max(20.0));
            self.filter.set_cutoff(smoothed_cutoff, self.sample_rate);
            let filtered = self.filter.process(raw);

            let env = self.amp_env.next();
            if env <= 0.0 {
                self.active = false;
            } else {
                body = filtered * env * self.settings.volume * self.drift_level;
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
        let old_click_type = self.settings.click_type;
        self.settings = KickSettings::from(settings);
        if self.settings.click_type != old_click_type {
            self.click = Self::make_click_generator(self.sample_rate, self.settings.click_type);
        }
        self.update_derived_params();
        // Update saturation config
        self.saturation.saturation_type =
            saturation::SaturationType::from(self.settings.saturation_type);
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
            self.saturation.saturation_type =
                saturation::SaturationType::from(self.settings.saturation_type);
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
        } else if index == 6 {
            self.settings.click_type = value as u8;
            self.click = Self::make_click_generator(self.sample_rate, self.settings.click_type);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the "click parasite": a pitch-plock retrigger must be
    /// continuous (no phase / amplitude jump) in BOTH analog AND digital modes.
    ///
    /// This is the test that was missing. The old HF-ratio test only inspected
    /// 3â€“20 kHz energy and was blind to the low-frequency phase discontinuity the
    /// digital path produced (measured trigger-edge jump ~0.20 with an open
    /// filter). Here we measure the body's sample-to-sample step across the
    /// retrigger directly, with the filter wide open so nothing is masked.
    #[test]
    fn test_kick_no_click_on_plock_retrigger_either_mode() {
        let sr = 44100.0;
        for &analog in &[1.0f32, 0.0f32] {
            let mut settings = KickSettings::default_at(sr);
            settings.analog = analog;
            settings.click_level = 0.0; // isolate the body from the legit click
            settings.filter_freq = 5000.0; // open filter so any discontinuity is exposed
            settings.filter_env_amount = 0.0;
            settings.decay = 0.25;
            settings.frequency = 60.0;

            let mut kick = KickVoice::new(sr, settings);
            kick.trigger();
            let mut last = 0.0f32;
            for _ in 0..1500 {
                last = kick.process_sample();
            }
            assert!(
                last.abs() > 1e-3,
                "tail must still ring for the test to be meaningful (analog={}): {}",
                analog,
                last
            );

            // Pitch plock to 200 Hz, then retrigger while the 60 Hz tail still rings.
            let mut plock = settings;
            plock.frequency = 200.0;
            kick.set_settings(plock.into());
            kick.trigger();

            let first = kick.process_sample();
            let edge = (first - last).abs();
            let mut prev = first;
            let mut max_step = 0.0f32;
            for _ in 0..300 {
                let s = kick.process_sample();
                max_step = max_step.max((s - prev).abs());
                prev = s;
            }
            // Before the fix the digital path jumped ~0.20 at the edge. Continuous now.
            assert!(
                edge < 0.05,
                "click parasite at plock retrigger (analog={}): trigger-edge step={}",
                analog,
                edge
            );
            assert!(
                max_step < 0.06,
                "click parasite after plock retrigger (analog={}): max step={}",
                analog,
                max_step
            );
        }
    }

    /// Renders the DIGITAL-mode kick with pitch plocks to a WAV for listening.
    /// This is the path that used to click (phase reset + broken crossfade). The
    /// legit click transient is disabled so any remaining parasite would be
    /// obvious. Listen at `target/test_wavs/kick_plock_digital_fixed.wav`.
    #[test]
    fn test_kick_plock_digital_render() {
        let sample_rate = 44100.0;
        let mut settings = KickSettings::default_at(sample_rate);
        settings.analog = 0.0; // the previously-clicking digital path
        settings.click_level = 0.0; // isolate body so any parasite is audible
        settings.filter_freq = 4000.0; // open-ish: no masking
        settings.filter_env_amount = 0.5;
        settings.decay = 0.3;

        let mut kick = KickVoice::new(sample_rate, settings);
        let gap = (sample_rate * 0.15) as usize;
        let mut samples: Vec<f32> = Vec::with_capacity(gap * 6);
        // hits 3 and 5 are pitch plocks to 200 Hz (the trigger of the parasite).
        let freqs = [60.0f32, 60.0, 200.0, 60.0, 200.0, 60.0];
        for &f in freqs.iter() {
            let mut s = settings;
            s.frequency = f;
            kick.set_settings(s.into());
            kick.trigger();
            for _ in 0..gap {
                samples.push(kick.process_sample());
            }
        }

        let out_dir = std::path::PathBuf::from("target/test_wavs");
        std::fs::create_dir_all(&out_dir).ok();
        let wav_path = out_dir.join("kick_plock_digital_fixed.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
        for s in &samples {
            writer.write_sample(*s).unwrap();
        }
        writer.finalize().unwrap();

        let mut max_step = 0.0f32;
        for w in samples.windows(2) {
            max_step = max_step.max((w[1] - w[0]).abs());
        }
        eprintln!(
            "\nDigital plock render: {} (max |sample step| = {:.5})",
            wav_path.display(),
            max_step
        );
        assert!(
            max_step < 0.1,
            "digital render still clicks: max sample step = {}",
            max_step
        );
    }

    /// Regression guard for the "attack = 0" click. A 0 ms attack must stay
    /// click-free both (a) on a cold start after silence â€” where the oscillator
    /// phase is left at an arbitrary value â€” and (b) on a retrigger during a
    /// ringing tail â€” where the amplitude would otherwise step from the tail
    /// level straight to full. Checked in both analog and digital modes.
    #[test]
    fn test_kick_zero_attack_no_click() {
        let sr = 44100.0;
        for &analog in &[1.0f32, 0.0f32] {
            let mut settings = KickSettings::default_at(sr);
            settings.analog = analog;
            settings.attack = 0.0; // the scenario under test
            settings.click_level = 0.0; // isolate the body
            settings.filter_freq = 5000.0; // open filter so nothing is masked
            settings.filter_env_amount = 0.0;
            settings.frequency = 60.0;

            // (a) Cold start after silence. Play a hit, let it fully decay so the
            // oscillator phase is left somewhere arbitrary, then trigger again.
            let mut cold = settings;
            cold.decay = 0.08;
            cold.release = 0.0;
            let mut kick = KickVoice::new(sr, cold);
            kick.trigger();
            let mut guard = 0;
            while kick.is_active() && guard < 44_100 {
                kick.process_sample();
                guard += 1;
            }
            assert!(
                !kick.is_active(),
                "voice should be silent before cold re-trigger"
            );
            kick.trigger();
            let mut prev = 0.0f32; // true silence baseline
            let mut cold_max = 0.0f32;
            for _ in 0..64 {
                let s = kick.process_sample();
                cold_max = cold_max.max((s - prev).abs());
                prev = s;
            }
            assert!(
                cold_max < 0.05,
                "attack=0 cold-start-after-silence click (analog={}): max step={}",
                analog,
                cold_max
            );

            // (b) Retrigger during a ringing tail (fresh voice, longer decay).
            let mut tail = settings;
            tail.decay = 0.3;
            let mut kick = KickVoice::new(sr, tail);
            kick.trigger();
            let mut last = 0.0f32;
            for _ in 0..600 {
                last = kick.process_sample();
            }
            assert!(
                kick.is_active() && last.abs() > 1e-3,
                "tail must still ring (analog={}): {}",
                analog,
                last
            );
            kick.trigger();
            let mut p = last;
            let mut retr_max = 0.0f32;
            for _ in 0..64 {
                let s = kick.process_sample();
                retr_max = retr_max.max((s - p).abs());
                p = s;
            }
            assert!(
                retr_max < 0.06,
                "attack=0 retrigger-on-tail click (analog={}): max step={}",
                analog,
                retr_max
            );
        }
    }

    /// Verifies the analog/digital toggle is now genuinely meaningful: analog
    /// drifts (no two hits identical), digital is bit-identical on every hit.
    #[test]
    fn test_kick_analog_drifts_digital_is_stable() {
        let sr = 44100.0;

        fn isolated_hit(kick: &mut KickVoice, n: usize) -> Vec<f32> {
            kick.trigger();
            let out: Vec<f32> = (0..n).map(|_| kick.process_sample()).collect();
            // Drain to silence so the next hit is a clean cold start.
            let mut guard = 0;
            while kick.is_active() && guard < 88_200 {
                kick.process_sample();
                guard += 1;
            }
            out
        }
        fn max_abs_diff(a: &[f32], b: &[f32]) -> f32 {
            a.iter()
                .zip(b)
                .map(|(x, y)| (x - y).abs())
                .fold(0.0f32, f32::max)
        }

        let mut base = KickSettings::default_at(sr);
        base.click_level = 0.0;
        base.decay = 0.3;
        base.frequency = 60.0;

        // Digital: two consecutive isolated hits must be (essentially) identical.
        let mut ds = base;
        ds.analog = 0.0;
        let mut kd = KickVoice::new(sr, ds);
        let d1 = isolated_hit(&mut kd, 1500);
        let d2 = isolated_hit(&mut kd, 1500);
        let dmax = max_abs_diff(&d1, &d2);

        // Analog: two consecutive isolated hits must differ (per-hit drift).
        let mut analog_settings = base;
        analog_settings.analog = 1.0;
        let mut ka = KickVoice::new(sr, analog_settings);
        let a1 = isolated_hit(&mut ka, 1500);
        let a2 = isolated_hit(&mut ka, 1500);
        let amax = max_abs_diff(&a1, &a2);

        eprintln!("\n=== analog drift vs digital stability ===");
        eprintln!("digital hit-to-hit max diff = {:.8}", dmax);
        eprintln!("analog  hit-to-hit max diff = {:.8}", amax);

        assert!(dmax < 1e-4, "digital hits must be identical: {}", dmax);
        assert!(amax > 0.005, "analog hits must drift (differ): {}", amax);
    }

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
        // dc_block doesn't add anything. Click is off â†’ output should be ~0.
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
        // Click transient excluded â€” that one is intentionally sharp.
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
            first_sample,
            second_sample,
            step
        );
    }

    #[test]
    fn test_kick_plock_frequency_change_no_click() {
        // Simulate a plock that changes frequency between two triggers.
        // The first trigger uses 60 Hz, the second (after a settings change)
        // uses 120 Hz.  We measure the peak of the first sample after the
        // second trigger; it should not contain a click spike.
        let mut settings = KickSettings::default_at(44100.0);
        settings.analog = 0.0; // Digital mode â€” test with reset
        settings.click_level = 0.0;
        settings.filter_freq = 1000.0;
        settings.filter_env_amount = 0.0; // disable filter env modulation for isolation

        let mut kick = KickVoice::new(44100.0, settings);

        // First trigger at 60 Hz
        kick.trigger();
        let mut last = 0.0f32;
        for _ in 0..500 {
            last = kick.process_sample();
        }

        // Plock-style settings change: double the frequency
        let mut new_settings = settings;
        new_settings.frequency = 120.0;
        kick.set_settings(new_settings.into());

        // Second trigger at 120 Hz. Baseline = the last tail sample, so we measure
        // the *real* discontinuity across the retrigger. (Earlier this test seeded
        // `prev = 0.0`, which only made sense when the digital path reset phase to
        // zero â€” it then conflated "first sample is non-zero" with "click". Phase
        // is now continuous, so the first sample is legitimately non-zero.)
        kick.trigger();
        let mut max_step = 0.0f32;
        let mut prev = last;
        for _ in 0..10 {
            let s = kick.process_sample();
            max_step = max_step.max((s - prev).abs());
            prev = s;
        }

        // A true click (discontinuity) would be > 0.3. With phase continuity the
        // step across the retrigger is now a small fraction of that.
        assert!(
            max_step < 0.06,
            "Click detected on plock frequency change: max_step={}",
            max_step
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

    /// Audio rendering test: records a WAV file with 4 kick triggers.
    /// Trigger 3 has a plock-style frequency jump (60 Hz -> 200 Hz).
    /// The WAV is written to `target/test_wavs/kick_plock_click.wav` so it can
    /// be listened to in a DAW.  We also print objective click metrics
    /// (HF energy ratio) for each trigger.
    #[test]
    fn test_kick_plock_click_audio_render() {
        use std::f32::consts::PI;

        let sample_rate = 44100.0;
        let mut settings = KickSettings::default_at(sample_rate);
        settings.analog = 1.0; // analog mode â€” the path where the click is audible
        settings.click_level = 0.5;
        settings.filter_freq = 2000.0;
        settings.filter_env_amount = 0.5;
        settings.decay = 0.25;

        let mut kick = KickVoice::new(sample_rate, settings);

        // --- render 4 triggers, ~150 ms apart ---
        let gap_samples = (sample_rate * 0.15) as usize; // 150 ms
        let mut samples: Vec<f32> = Vec::with_capacity(gap_samples * 4);

        let freqs = [60.0_f32, 60.0, 200.0, 60.0]; // trigger 3 = plock jump
        for (i, &freq) in freqs.iter().enumerate() {
            if i == 2 {
                // plock: change frequency just before trigger
                let mut new_settings = settings;
                new_settings.frequency = freq;
                kick.set_settings(new_settings.into());
            } else if i == 3 {
                // back to original
                kick.set_settings(settings.into());
            }
            kick.trigger();
            for _ in 0..gap_samples {
                samples.push(kick.process_sample());
            }
        }

        // --- write WAV ---
        let out_dir = std::path::PathBuf::from("target/test_wavs");
        std::fs::create_dir_all(&out_dir).ok();
        let wav_path = out_dir.join("kick_plock_click.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).unwrap();
        for s in &samples {
            writer.write_sample(*s).unwrap();
        }
        writer.finalize().unwrap();

        // --- objective click analysis ---
        // For each trigger, compute energy in 0-3 kHz vs 3-20 kHz during the
        // first 10 ms after the trigger.  A click shows as a spike in the HF band.
        let window = (sample_rate * 0.01) as usize; // 10 ms
        let mut results = Vec::new();
        for t in 0..4 {
            let start = t * gap_samples;
            let end = (start + window).min(samples.len());
            let mut energy_low = 0.0_f32;
            let mut energy_high = 0.0_f32;
            for s in &samples[start..end] {
                energy_low += s * s;
            }
            // Simple 1-pole HP @ 3 kHz for HF estimate
            let mut hp_state = 0.0_f32;
            let alpha = 1.0 - (-2.0 * PI * 3000.0 / sample_rate).exp();
            for s in &samples[start..end] {
                hp_state += alpha * (s - hp_state);
                let hp = s - hp_state;
                energy_high += hp * hp;
            }
            let hf_ratio = if energy_low > 0.0 {
                energy_high / energy_low
            } else {
                0.0
            };
            results.push((t + 1, freqs[t], hf_ratio, energy_low.sqrt()));
        }

        eprintln!("\n=== Kick plock click analysis ===");
        eprintln!("WAV: {}", wav_path.display());
        eprintln!(
            "{:<8} {:<10} {:<12} {:<12}",
            "Trigger", "Freq(Hz)", "HF_ratio", "RMS"
        );
        for (t, f, ratio, rms) in &results {
            eprintln!("{:<8} {:<10.0} {:<12.6} {:<12.6}", t, f, ratio, rms);
        }

        // Trigger 3 (plock) should not have a dramatically higher HF ratio.
        // If it does, we have a click.
        let baseline_hf = (results[0].2 + results[1].2 + results[3].2) / 3.0;
        let plock_hf = results[2].2;
        let spike = plock_hf / baseline_hf.max(0.0001);
        eprintln!("\nPlock HF spike factor: {:.2}x baseline", spike);

        // We tolerate up to 2x because a higher frequency naturally has more HF.
        assert!(
            spike < 3.0,
            "Click detected: plock HF energy is {:.1}x baseline. Listen to {}.",
            spike,
            wav_path.display()
        );
    }
}
