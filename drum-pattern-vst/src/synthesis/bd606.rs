//! BD606 — multisample TR-606 bass drum.
//!
//! Plays one of the 8 embedded hits per trigger (see `sample_bank.rs`):
//! - Analog Mode ON: random pick without immediate repeat (the machine's
//!   hit-to-hit variability).
//! - Analog Mode OFF: always the user-selected sample (1..8).
//!
//! Pitch is relative (-24..+24 semitones + fine cents), the amp envelope
//! uses attack/decay fractions of the played hit (bypassed in One Shot mode),
//! and an additive filter envelope drives a one-pole low-pass.

use super::{dsp, sample_bank, saturation, settings::bd606::Bd606Settings, Voice, VoiceSettings};

/// Anti-click floor for the amplitude attack (a true 0 ms attack is a step).
const MIN_AMP_ATTACK_MS: f32 = 0.3;
/// Full-scale amp attack (attack = 1.0) as an ABSOLUTE fade-in time. The attack
/// parameter is a fraction of THIS ceiling, not of the sample length — on very
/// short hits (a closed hi-hat) a fraction of the sample gave no usable musical
/// range and just crushed the transient. 80 ms is a soft-attack ceiling for drums.
const MAX_AMP_ATTACK_SECS: f32 = 0.08;
const LEGACY_ROOT_FREQ: f32 = 60.0;
/// Depth of the additive filter envelope (Hz at amount = 1).
const FILTER_ENV_DEPTH_HZ: f32 = 8000.0;

pub struct Bd606Voice {
    settings: Bd606Settings,
    sample_rate: f32,

    /// Playback position in SOURCE samples (fractional).
    pos: f32,
    /// Playback stop position in SOURCE samples (End parameter).
    end_pos: f32,
    /// Source-sample increment per output sample (rate ratio × pitch).
    step: f32,
    current_hit: usize,
    last_hit: usize,
    /// xorshift32 state — seeded at construction, never reseeded on trigger
    /// (retrigger continuity convention).
    rng: u32,

    amp_env: dsp::DecayReleaseEnvelope,
    filter: dsp::OnePoleFilter,
    filter_env: dsp::ExpDecayEnvelope,
    saturation: saturation::SaturationConfig,
    dc_block: dsp::DcBlocker,

    active: bool,
}

impl Bd606Voice {
    pub fn new(sample_rate: f32, settings: Bd606Settings) -> Self {
        let decay = settings.decay.max(0.01).min(5.0);
        let mut amp_env = dsp::DecayReleaseEnvelope::new(
            sample_rate,
            settings.decay_curve,
            decay,
            settings.release_curve,
            settings.release.max(0.001),
        )
        .with_attack_ms((settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
        amp_env.set_hold(settings.hold);

        let filter = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        let filter_env =
            dsp::ExpDecayEnvelope::new(sample_rate, settings.decay_curve, 0.15).with_attack_ms(0.3);

        let mut voice = Self {
            settings,
            sample_rate,
            pos: 0.0,
            end_pos: f32::MAX,
            step: 1.0,
            current_hit: 0,
            last_hit: 0,
            rng: 0x6060_0001,
            amp_env,
            filter,
            filter_env,
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::None,
                amount: 0.0,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
                compensation_gain: 1.0,
            },
            dc_block: dsp::DcBlocker::default(),
            active: false,
        };
        let decay_secs = voice.filter_env_decay_seconds();
        voice.filter_env.set_decay(decay_secs);
        let amp_decay = voice.amp_decay_secs();
        voice.amp_env.set_decay(amp_decay);
        let amp_attack_ms = voice.amp_attack_secs() * 1000.0;
        voice
            .amp_env
            .set_attack_ms(amp_attack_ms.max(MIN_AMP_ATTACK_MS));
        voice
    }

    fn one_shot(&self) -> bool {
        self.settings.one_shot > 0.5
    }

    fn analog_mode(&self) -> bool {
        self.settings.analog_mode > 0.5
    }

    fn pitch_ratio(&self) -> f32 {
        let coarse = if self.settings.pitch_format_version >= 0.5 {
            self.settings.frequency
        } else if self.settings.frequency > 0.0 {
            12.0 * (self.settings.frequency / LEGACY_ROOT_FREQ).log2()
        } else {
            0.0
        };
        let semis =
            coarse.clamp(-24.0, 24.0) + self.settings.fine_tune.clamp(-100.0, 100.0) / 100.0;
        2f32.powf(semis / 12.0)
    }

    /// Duration of the selected hit once the pitch (playback rate) is applied.
    fn played_secs(&self) -> f32 {
        let bank = sample_bank::bd606();
        let hit_len = bank.hits[self.current_hit].len().max(1) as f32;
        hit_len / bank.source_rate / self.pitch_ratio()
    }

    /// Amp attack in seconds: an ABSOLUTE fade-in time (fraction of
    /// `MAX_AMP_ATTACK_SECS`), independent of the sample length.
    fn amp_attack_secs(&self) -> f32 {
        self.settings.attack.clamp(0.0, 1.0) * MAX_AMP_ATTACK_SECS
    }

    /// Amp decay in seconds: the parameter is a FRACTION of the played
    /// sample length.
    fn amp_decay_secs(&self) -> f32 {
        self.settings.decay.clamp(0.01, 1.0) * self.played_secs()
    }

    /// Filter envelope decay in seconds: the parameter is a FRACTION of the
    /// played sample length (pitch included), so the sweep tracks the hit.
    fn filter_env_decay_seconds(&self) -> f32 {
        self.settings.filter_env_decay.clamp(0.01, 1.0) * self.played_secs()
    }

    /// End of playback as a fraction of the sample length. Settings saved
    /// before the End parameter existed (legacy pitch marker) play the
    /// whole sample.
    fn end_frac(&self) -> f32 {
        if self.settings.pitch_format_version >= 0.5 {
            self.settings.end.clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Random hit without immediate repeat.
    fn next_random_hit(&mut self) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        let mut h = (self.rng >> 8) as usize % sample_bank::HIT_COUNT;
        if h == self.last_hit {
            h = (h + 1) % sample_bank::HIT_COUNT;
        }
        h
    }
}

impl Voice for Bd606Voice {
    fn trigger(&mut self) {
        self.active = true;

        self.current_hit = if self.analog_mode() {
            self.next_random_hit()
        } else {
            (self.settings.sample_index.round() as usize).clamp(1, sample_bank::HIT_COUNT) - 1
        };
        self.last_hit = self.current_hit;
        let bank = sample_bank::bd606();
        let rate_ratio = bank.source_rate / self.sample_rate;
        self.step = rate_ratio * self.pitch_ratio();
        // Start/End: fractions of the selected sample's length.
        let hit_len = bank.hits[self.current_hit].len();
        let start = self.settings.start_offset.clamp(0.0, 1.0);
        let end = self.end_frac().max(start + 0.005);
        self.pos = start * hit_len.saturating_sub(1) as f32;
        self.end_pos = end * hit_len as f32;

        self.amp_env.set_decay(self.amp_decay_secs());
        let amp_attack_ms = self.amp_attack_secs() * 1000.0;
        self.amp_env
            .set_attack_ms(amp_attack_ms.max(MIN_AMP_ATTACK_MS));
        self.amp_env.set_release(self.settings.release.max(0.001));
        self.amp_env.trigger();
        let decay_secs = self.filter_env_decay_seconds();
        self.filter_env.set_decay(decay_secs);
        self.filter_env.trigger();
    }

    fn trigger_hard(&mut self) {
        self.trigger();
        self.amp_env.trigger_hard();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let bank = sample_bank::bd606();
        let hit = &bank.hits[self.current_hit];
        if hit.len() < 2 {
            self.active = false;
            return 0.0;
        }

        let idx = self.pos as usize;
        if idx + 1 >= hit.len() || self.pos + 1.0 >= self.end_pos {
            self.active = false;
            return 0.0;
        }
        let frac = self.pos - idx as f32;
        let raw = hit[idx] + frac * (hit[idx + 1] - hit[idx]);
        self.pos += self.step;

        let amp = if self.one_shot() {
            1.0
        } else {
            let amp = self.amp_env.next();
            if amp <= 0.0 {
                self.active = false;
                return 0.0;
            }
            amp
        };

        let mut out = raw * amp;

        // Filter — additive envelope: Cutoff at rest + (envelope × amount × depth)
        let filter_env_val = self.filter_env.next();
        let filter_freq = self.settings.filter_freq.max(20.0).min(20000.0);
        let effective_freq =
            filter_freq + filter_env_val * self.settings.filter_env_amount * FILTER_ENV_DEPTH_HZ;
        self.filter
            .set_cutoff(effective_freq.max(20.0).min(20000.0), self.sample_rate);
        out = self.filter.process(self.saturation.process_at(true, out));

        // Volume post-saturation: the knob sets the final level, not the drive.
        self.dc_block
            .process(self.saturation.process_at(false, out))
            * self.settings.volume
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.pos = 0.0;
        self.amp_env.reset();
        self.filter_env.reset();
        self.filter.reset();
        self.dc_block.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = Bd606Settings::from(settings);

        // Update envelopes via setters — do NOT recreate to preserve tail state
        let amp_decay = self.amp_decay_secs();
        self.amp_env.set_decay(amp_decay);
        let amp_attack_ms = self.amp_attack_secs() * 1000.0;
        self.amp_env
            .set_attack_ms(amp_attack_ms.max(MIN_AMP_ATTACK_MS));
        self.amp_env.set_release(self.settings.release.max(0.001));
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
        self.amp_env.set_hold(self.settings.hold);

        let decay_secs = self.filter_env_decay_seconds();
        self.filter_env.set_decay(decay_secs);
        self.filter_env.set_curve(self.settings.decay_curve);

        let filter_freq = self.settings.filter_freq.max(20.0).min(20000.0);
        self.filter.set_cutoff(filter_freq, self.sample_rate);

        self.saturation.saturation_type =
            saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
        self.saturation.pre_filter = self.settings.saturation_pre_filter > 0.5;
        self.saturation.update_compensation();
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.settings.analog_mode = value,
            1 => self.settings.sample_index = value,
            2 => self.settings.one_shot = value,
            3 => self.settings.start_offset = value,
            11 => self.settings.end = value,
            4 => {
                self.settings.saturation_type = value as u8;
                self.saturation.saturation_type =
                    saturation::SaturationType::from(self.settings.saturation_type);
            }
            5 => {
                self.settings.saturation_amount = value;
                self.saturation.amount = value;
            }
            6 => {
                self.settings.saturation_mix = value;
                self.saturation.mix = value;
            }
            7 => {
                self.settings.saturation_output_gain = value;
                self.saturation.output_gain = value;
            }
            8 => {
                self.settings.saturation_pre_filter = value;
                self.saturation.pre_filter = value > 0.5;
            }
            _ => {}
        }
        self.saturation.update_compensation();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice_with(settings: VoiceSettings) -> Bd606Voice {
        Bd606Voice::new(44100.0, Bd606Settings::from(settings))
    }

    #[test]
    fn produces_sound_on_trigger() {
        let mut voice = voice_with(VoiceSettings::bd606());
        voice.trigger();
        let mut peak = 0.0f32;
        for _ in 0..44100 {
            peak = peak.max(voice.process_sample().abs());
        }
        assert!(peak > 0.05, "BD606 should produce sound, peak {peak}");
    }

    #[test]
    fn output_stays_finite_and_stops() {
        let mut voice = voice_with(VoiceSettings::bd606());
        for _ in 0..20 {
            voice.trigger();
            for _ in 0..44100 * 2 {
                let s = voice.process_sample();
                assert!(s.is_finite(), "non-finite sample");
            }
        }
        // After the hits decay the voice must go silent.
        let mut tail = 0.0f32;
        for _ in 0..44100 {
            tail = tail.max(voice.process_sample().abs());
        }
        assert!(
            tail < 1e-4,
            "voice should be silent after hits, tail {tail}"
        );
        assert!(!voice.is_active());
    }

    #[test]
    fn fixed_sample_mode_repeats_the_same_hit() {
        let mut settings = VoiceSettings::bd606();
        settings.special[0] = 0.0; // analog mode off
        settings.special[1] = 3.0; // sample 3
        settings.special[2] = 1.0; // one shot: capture the raw hit
        let mut voice = voice_with(settings);

        let capture = |voice: &mut Bd606Voice| -> Vec<f32> {
            // reset() between captures so the analog-continuity state
            // (filter, DC blocker) doesn't leak from the previous hit.
            voice.reset();
            voice.trigger();
            assert_eq!(voice.current_hit, 2, "fixed mode must pick sample 3");
            (0..4000).map(|_| voice.process_sample()).collect()
        };
        let a = capture(&mut voice);
        let b = capture(&mut voice);
        assert_eq!(a, b, "fixed sample mode must be bit-identical");
    }

    #[test]
    fn analog_mode_never_repeats_the_same_hit_twice() {
        let mut settings = VoiceSettings::bd606();
        settings.special[0] = 1.0;
        let mut voice = voice_with(settings);
        let mut prev = usize::MAX;
        for _ in 0..64 {
            voice.trigger();
            assert_ne!(
                voice.current_hit, prev,
                "analog mode repeated a hit immediately"
            );
            prev = voice.current_hit;
        }
    }

    #[test]
    fn pitch_changes_playback_duration() {
        let mut settings = VoiceSettings::bd606();
        settings.special[2] = 1.0; // one shot: duration = sample length / rate

        let duration = |freq: f32| -> usize {
            let mut s = settings;
            s.frequency = freq;
            let mut voice = voice_with(s);
            voice.trigger();
            let mut n = 0;
            while voice.is_active() && n < 44100 * 4 {
                voice.process_sample();
                n += 1;
            }
            n
        };

        let normal = duration(0.0);
        let octave_up = duration(12.0);
        assert!(
            octave_up < normal * 3 / 4,
            "octave up should shorten the hit (normal={normal}, up={octave_up})"
        );
    }

    #[test]
    fn one_shot_ignores_amp_envelope() {
        let mut settings = VoiceSettings::bd606();
        settings.decay = 0.001; // near-instant amp decay
        settings.release = 0.0;

        let mut gated = voice_with(settings);
        gated.trigger();
        let mut gated_energy = 0.0f32;
        for _ in 0..44100 {
            gated_energy += gated.process_sample().abs();
        }

        settings.special[2] = 1.0; // one shot
        let mut one_shot = voice_with(settings);
        one_shot.trigger();
        let mut one_shot_energy = 0.0f32;
        for _ in 0..44100 {
            one_shot_energy += one_shot.process_sample().abs();
        }

        assert!(
            one_shot_energy > gated_energy * 2.0,
            "one shot should play the full hit (gated={gated_energy}, one_shot={one_shot_energy})"
        );
    }

    #[test]
    fn start_offset_is_a_fraction_of_the_sample_length() {
        let mut settings = VoiceSettings::bd606();
        settings.special[0] = 0.0; // fixed sample
        settings.special[1] = 1.0; // sample 1
        settings.special[2] = 1.0; // one shot: raw playback
        settings.special[3] = 0.5; // start halfway into the hit
        let mut voice = voice_with(settings);
        voice.trigger();

        let bank = sample_bank::bd606();
        let hit = &bank.hits[0];
        let start = (hit.len() - 1) / 2;
        let captured: Vec<f32> = (0..64).map(|_| voice.process_sample()).collect();
        let mut max_err = 0.0f32;
        for (i, &c) in captured.iter().enumerate() {
            max_err = max_err.max((c - hit[start + i]).abs());
        }
        assert!(
            max_err < 0.05,
            "start 0.5 should begin playback mid-sample (err {max_err})"
        );
    }

    #[test]
    fn filter_decay_tracks_the_played_sample_length() {
        // 0.5 × 1 s of sample at native pitch, halved one octave up.
        let mut settings = VoiceSettings::bd606();
        settings.filter_env_decay = 0.5;
        let mut voice = voice_with(settings);
        let d_native = voice.filter_env_decay_seconds();
        settings.frequency = 12.0;
        voice.set_settings(settings);
        let d_octave = voice.filter_env_decay_seconds();
        assert!((d_native - 0.5).abs() < 0.01, "native pitch: {d_native}s");
        assert!((d_octave - 0.25).abs() < 0.01, "octave up: {d_octave}s");
    }

    #[test]
    fn amp_decay_tracks_length_attack_is_absolute() {
        let mut settings = VoiceSettings::bd606();
        settings.attack = 0.1;
        settings.decay = 0.5;
        let mut voice = voice_with(settings);
        // Attack is an absolute fade-in time; decay is a fraction of the played length.
        assert!((voice.amp_attack_secs() - 0.1 * MAX_AMP_ATTACK_SECS).abs() < 1e-4);
        assert!((voice.amp_decay_secs() - 0.5).abs() < 0.01);

        settings.frequency = 12.0;
        voice.set_settings(settings);
        // Pitch up halves the played length → decay follows, attack does not.
        assert!((voice.amp_attack_secs() - 0.1 * MAX_AMP_ATTACK_SECS).abs() < 1e-4);
        assert!((voice.amp_decay_secs() - 0.25).abs() < 0.01);
    }

    #[test]
    fn fine_tune_adds_cents_to_relative_pitch() {
        let mut settings = VoiceSettings::bd606();
        settings.special[9] = 100.0;
        let voice = voice_with(settings);
        let expected = 2f32.powf(1.0 / 12.0);
        assert!((voice.pitch_ratio() - expected).abs() < 1e-6);
    }

    #[test]
    fn legacy_hz_pitch_keeps_native_rate() {
        let mut settings = VoiceSettings::bd606();
        settings.frequency = 60.0;
        settings.special[10] = 0.0;
        let voice = voice_with(settings);
        assert!((voice.pitch_ratio() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn end_truncates_playback() {
        let mut settings = VoiceSettings::bd606();
        settings.special[2] = 1.0; // one shot: duration = played region / rate
        let duration = |end: f32| -> usize {
            let mut s = settings;
            s.special[11] = end;
            let mut voice = voice_with(s);
            voice.trigger();
            let mut n = 0;
            while voice.is_active() && n < 44100 * 4 {
                voice.process_sample();
                n += 1;
            }
            n
        };

        let full = duration(1.0);
        let half = duration(0.5);
        assert!(
            half < full * 3 / 4,
            "end 0.5 should halve the hit (full={full}, half={half})"
        );
    }

    #[test]
    fn legacy_settings_without_end_play_the_full_sample() {
        let mut settings = VoiceSettings::bd606();
        settings.special[2] = 1.0;
        settings.special[10] = 0.0; // legacy marker
        settings.special[11] = 0.0; // no End in the old blob
        let mut voice = voice_with(settings);
        voice.trigger();
        let mut n = 0;
        while voice.is_active() && n < 44100 * 4 {
            voice.process_sample();
            n += 1;
        }
        assert!(
            n > 40000,
            "legacy settings should play the whole 1 s hit, stopped at {n}"
        );
    }
}
