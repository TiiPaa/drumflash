//! One-Shot ([243]) — the lane's own sample file, played start to finish.
//!
//! Rift lifts a slice out of a long texture; One-Shot is its simpler sibling:
//! no embedded content, no offset — the whole user file ([228] lane textures,
//! same pool, same lane rules) is the sound. It is shaped by a full A-H-D
//! amplitude envelope, a full A-H-D pitch envelope (depth in semitones, each
//! ramp with its own curve), a resonant LP/HP/BP filter with an A-H-D
//! envelope, reverse playback, and the shared saturation chain.
//!
//! A lane WITHOUT a file stays inert: there is no embedded fallback, the file
//! IS the instrument.
//!
//! Retrigger follows the [179] contract: read position, filter and DC blocker
//! restart from a clean state, `RetrigDeclick` (3 ms) absorbs the jump.

use super::{dsp, sample_bank, saturation, settings::oneshot::OneShotSettings, Voice, VoiceSettings};

/// Anti-click floor for the amplitude attack (a true 0 ms attack is a step).
const MIN_AMP_ATTACK_MS: f32 = 0.3;
/// Full-scale amp attack as an ABSOLUTE fade-in time (same reasoning as Rift).
const MAX_AMP_ATTACK_SECS: f32 = 0.08;
/// Amp decay is an ABSOLUTE time, long enough at the top to let any sane
/// one-shot ring to its own end (the registry slider spans 0.005 .. 10 s).
const MAX_DECAY_SECS: f32 = 10.0;
const MIN_DECAY_SECS: f32 = 0.005;
/// Hold is an absolute time too, capped at one second.
const MAX_HOLD_SECS: f32 = 1.0;
/// Top of the filter sweep — the plugin's own convention (`buzz.rs`): the
/// envelope opens the cutoff toward it and the cutoff falls back afterwards.
const FILTER_OPEN_HZ: f32 = 20000.0;
/// Anti-click floor for the filter- and pitch-envelope attacks.
const MIN_ENV_ATTACK_S: f32 = 0.0005;
/// Pitch envelope times: attack/hold caps from the registry, decay cap here.
const MAX_PITCH_ENV_DECAY_S: f32 = 1.5;
/// The biquad is re-tuned every N samples rather than every sample (rift.rs).
const FILTER_UPDATE_SAMPLES: u32 = 8;

pub struct OneShotVoice {
    settings: OneShotSettings,
    sample_rate: f32,

    /// Read position in SOURCE samples (fractional).
    pos: f32,
    /// End of the file in SOURCE samples (the whole file is the window).
    win_end: f32,
    /// Source-sample increment per output sample, pitch modulation EXCLUDED —
    /// the pitch envelope scales it per sample.
    base_step: f32,
    /// The lane's file, held alive by its `Arc` for the length of the hit.
    source: Option<std::sync::Arc<sample_bank::TextureBank>>,
    /// The instance's lane textures and this voice's lane; `None` = test
    /// harness without a pool (the voice then stays inert unless a test
    /// publishes a bank through the pool it was given).
    pool: Option<(std::sync::Arc<sample_bank::TexturePool>, usize)>,

    amp_env: dsp::DecayReleaseEnvelope,
    /// Elapsed time in the A-H-D pitch envelope — a manual ramp, because
    /// attack and decay each carry their own bipolar curve.
    pitch_env_time: f32,
    filter: dsp::Biquad,
    /// Right-channel twin of `filter` (stereo files), same tuning.
    filter_r: dsp::Biquad,
    /// Elapsed time in the A-H-D filter envelope, same shape as `rift.rs`.
    filter_env_time: f32,
    filter_tick: u32,
    saturation: saturation::SaturationConfig,
    dc_block: dsp::DcBlocker,
    dc_block_r: dsp::DcBlocker,
    /// One declick per channel (a stereo file's channels end on different
    /// residues).
    declick: [dsp::RetrigDeclick; 2],
    last_out: [f32; 2],

    active: bool,
}

impl OneShotVoice {
    pub fn new(sample_rate: f32, settings: OneShotSettings) -> Self {
        let mut amp_env = dsp::DecayReleaseEnvelope::new(
            sample_rate,
            settings.decay_curve,
            settings.decay.clamp(MIN_DECAY_SECS, MAX_DECAY_SECS),
            settings.release_curve,
            settings.release.max(0.001),
        )
        .with_attack_ms((settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
        amp_env.set_hold(settings.hold);

        let mut voice = Self {
            settings,
            sample_rate,
            pos: 0.0,
            win_end: 0.0,
            base_step: 1.0,
            source: None,
            pool: None,
            amp_env,
            pitch_env_time: 1.0e6,
            filter: dsp::Biquad::new(),
            filter_r: dsp::Biquad::new(),
            filter_env_time: 1.0e6,
            filter_tick: 0,
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::None,
                amount: 0.0,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
                compensation_gain: 1.0,
            },
            dc_block: dsp::DcBlocker::default(),
            dc_block_r: dsp::DcBlocker::default(),
            declick: [dsp::RetrigDeclick::new(sample_rate); 2],
            last_out: [0.0; 2],
            active: false,
        };
        voice.apply_filter(0.0);
        voice.sync_saturation();
        voice
    }

    /// The saturation pack from the settings - shared by construction and
    /// `set_settings` (the test harness never calls `set_settings`).
    fn sync_saturation(&mut self) {
        self.saturation.saturation_type =
            saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
        self.saturation.pre_filter = self.settings.saturation_pre_filter > 0.5;
        self.saturation.update_compensation();
    }

    /// Linear-interpolated read; the caller guarantees `pos + 1 < data.len()`.
    #[inline]
    fn read(data: &[f32], pos: f32) -> f32 {
        let idx = pos as usize;
        let frac = pos - idx as f32;
        data[idx] + frac * (data[idx + 1] - data[idx])
    }

    /// One channel of the file at `pos`. `channel`: `None` = the mono material
    /// (a stereo file mixed down at read time), `Some(false)` = left,
    /// `Some(true)` = right. Mono banks answer the same for all three.
    #[inline]
    fn sample(bank: &sample_bank::TextureBank, channel: Option<bool>, pos: f32) -> f32 {
        match (channel, bank.right.as_deref()) {
            (Some(true), Some(right)) => Self::read(right, pos),
            (None, Some(right)) => 0.5 * (Self::read(&bank.data, pos) + Self::read(right, pos)),
            _ => Self::read(&bank.data, pos),
        }
    }

    /// The Stereo switch is on AND the file has two channels.
    fn stereo_file(&self) -> bool {
        self.settings.stereo > 0.5
            && self
                .source
                .as_ref()
                .map(|b| b.right.is_some())
                .unwrap_or(false)
    }

    fn reverse(&self) -> bool {
        self.settings.reverse > 0.5
    }

    /// Resting playback ratio: coarse semitones plus fine cents.
    fn pitch_ratio(&self) -> f32 {
        let semis = self.settings.frequency.clamp(-24.0, 24.0)
            + self.settings.pitch_fine.clamp(-100.0, 100.0) / 100.0;
        2f32.powf(semis / 12.0)
    }

    fn amp_attack_secs(&self) -> f32 {
        self.settings.attack.clamp(0.0, 1.0) * MAX_AMP_ATTACK_SECS
    }

    fn amp_decay_secs(&self) -> f32 {
        self.settings.decay.clamp(MIN_DECAY_SECS, MAX_DECAY_SECS)
    }

    fn hold_secs(&self) -> f32 {
        self.settings.hold.clamp(0.0, MAX_HOLD_SECS)
    }

    fn filter_env_decay_secs(&self) -> f32 {
        self.settings.filter_env_decay.clamp(MIN_DECAY_SECS, MAX_DECAY_SECS)
    }

    /// Generic A-H-D ramp at time `t`, each stage shaped by its own bipolar
    /// curve (the pitch and filter envelopes share this shape).
    fn ahd_value(t: f32, attack: f32, hold: f32, decay: f32, atk_curve: f32, dec_curve: f32) -> f32 {
        let atk = attack.max(MIN_ENV_ATTACK_S);
        let hold = hold.max(0.0);
        let dec = decay.max(MIN_DECAY_SECS);
        if t < atk {
            dsp::shape_curve(t / atk, atk_curve)
        } else if t < atk + hold {
            1.0
        } else {
            let p = ((t - atk - hold) / dec).clamp(0.0, 1.0);
            dsp::shape_curve(1.0 - p, dec_curve)
        }
    }

    /// A-H-D pitch envelope value (0..1); the caller scales it by the depth.
    fn pitch_env_value(&self) -> f32 {
        Self::ahd_value(
            self.pitch_env_time,
            self.settings.pitch_env_attack,
            self.settings.pitch_env_hold,
            self.settings
                .pitch_env_decay
                .clamp(MIN_DECAY_SECS, MAX_PITCH_ENV_DECAY_S),
            self.settings.pitch_env_atk_curve,
            self.settings.pitch_env_dec_curve,
        )
    }

    /// A-H-D filter envelope value (0..1), as in `rift.rs`.
    fn filter_env_value(&self) -> f32 {
        Self::ahd_value(
            self.filter_env_time,
            self.settings.filter_attack,
            self.settings.filter_hold,
            self.filter_env_decay_secs(),
            self.settings.filter_atk_curve,
            self.settings.filter_dec_curve,
        )
    }

    /// Cutoff for the current envelope value — the plugin's own convention:
    /// the envelope sweeps the cutoff EXPONENTIALLY from its setting up toward
    /// 20 kHz; `Filter Env` (0..1) says how far.
    fn filter_cutoff(&self, env: f32) -> f32 {
        let base = self.settings.filter_freq.clamp(20.0, FILTER_OPEN_HZ);
        let amt = (env * self.settings.filter_env_amount).clamp(0.0, 1.0);
        (base * (FILTER_OPEN_HZ / base).powf(amt)).clamp(20.0, FILTER_OPEN_HZ)
    }

    fn apply_filter(&mut self, env: f32) {
        let cutoff = self.filter_cutoff(env);
        let q = self.settings.resonance.clamp(0.5, 20.0);
        match self.settings.filter_type.round() as i32 {
            1 => self.filter.set_highpass(cutoff, q, self.sample_rate),
            2 => self.filter.set_bandpass(cutoff, q, self.sample_rate),
            _ => self.filter.set_lowpass(cutoff, q, self.sample_rate),
        }
        self.filter_r.copy_coefficients_from(&self.filter);
    }

    /// One output frame. `stereo` = a stereo file plays its two channels under
    /// the SAME envelopes and filter tuning. Mono returns the left twice.
    fn render(&mut self, stereo: bool) -> (f32, f32) {
        if !self.active {
            let t0 = self.declick[0].next();
            let t1_own = self.declick[1].next();
            let t1 = if stereo { t1_own } else { t0 };
            self.last_out = [t0, t1];
            return (t0, t1);
        }

        let Some(bank) = self.source.clone() else {
            self.active = false;
            return (0.0, 0.0);
        };
        let data_len = bank.data.len();
        let past_end = if self.reverse() {
            self.pos - 1.0 <= 0.0
        } else {
            self.pos + 1.0 >= self.win_end
        };
        if past_end {
            return self.cut();
        }
        let idx = self.pos as usize;
        if self.pos < 0.0 || idx + 1 >= data_len {
            return self.cut();
        }

        let two_channels = self.stereo_file();
        let (ch_l, ch_r) = if two_channels {
            (Some(false), Some(true))
        } else {
            (None, None)
        };
        let raw_l = Self::sample(&bank, ch_l, self.pos);
        let raw_r = if stereo {
            Self::sample(&bank, ch_r, self.pos)
        } else {
            raw_l
        };

        // Pitch: the A-H-D envelope scales the playback rate by its depth.
        let semis = self.pitch_env_value() * self.settings.pitch_env.clamp(-24.0, 24.0);
        let advance = if semis.abs() > 1e-4 {
            self.base_step * 2f32.powf(semis / 12.0)
        } else {
            self.base_step
        };
        self.pos += if self.reverse() { -advance } else { advance };
        self.pitch_env_time += 1.0 / self.sample_rate;

        let amp = self.amp_env.next();
        if amp <= 0.0 {
            return self.cut();
        }

        if self.filter_tick == 0 {
            let env = self.filter_env_value();
            self.apply_filter(env);
        }
        self.filter_env_time += 1.0 / self.sample_rate;
        self.filter_tick = (self.filter_tick + 1) % FILTER_UPDATE_SAMPLES;

        let l = self.render_channel(0, raw_l, amp);
        let r = if stereo {
            self.render_channel(1, raw_r, amp)
        } else {
            l
        };
        let t0 = self.declick[0].next();
        let t1_own = self.declick[1].next();
        let t1 = if stereo { t1_own } else { t0 };
        let (out_l, out_r) = (l + t0, r + t1);
        self.last_out = [out_l, out_r];
        (out_l, out_r)
    }

    /// End the voice NOW - envelope over or file exhausted - without a step:
    /// the filter and DC blocker still hold a residue when the amp envelope
    /// reaches zero, and dropping to exact silence in one sample was a tick.
    fn cut(&mut self) -> (f32, f32) {
        self.active = false;
        for ch in 0..2 {
            self.declick[ch].arm(self.last_out[ch]);
        }
        let t0 = self.declick[0].next();
        let t1 = self.declick[1].next();
        self.last_out = [t0, t1];
        (t0, t1)
    }

    /// The Distortion block for one channel: the saturation pack, before or
    /// after the filter on the **Pre-Filter** switch ([232]).
    #[inline]
    fn render_channel(&mut self, ch: usize, raw: f32, amp: f32) -> f32 {
        let x = raw * amp;
        let pre = self.saturation.process_at(true, x);
        let filtered = if ch == 0 {
            self.filter.process(pre)
        } else {
            self.filter_r.process(pre)
        };
        let post = self.saturation.process_at(false, filtered);
        let dc = if ch == 0 {
            self.dc_block.process(post)
        } else {
            self.dc_block_r.process(post)
        };
        dc * self.settings.volume
    }
}

impl Voice for OneShotVoice {
    fn trigger(&mut self) {
        // [179] The read restarts from scratch; the declick covers the jump.
        for ch in 0..2 {
            self.declick[ch].arm(self.last_out[ch]);
        }
        self.filter.reset();
        self.filter_r.reset();
        self.dc_block.reset();
        self.dc_block_r.reset();

        // The lane's file is the whole instrument: no file, no sound.
        let source = self
            .pool
            .as_ref()
            .and_then(|(pool, lane)| pool.get(*lane));
        let Some(bank) = source.clone() else {
            self.active = false;
            self.source = None;
            return;
        };
        let len = bank.data.len();
        if len < 4 {
            self.active = false;
            self.source = None;
            return;
        }
        self.active = true;
        self.win_end = (len - 2) as f32;
        self.base_step = (bank.source_rate.max(1.0) / self.sample_rate) * self.pitch_ratio();
        self.pos = if self.reverse() { self.win_end } else { 0.0 };

        self.amp_env.set_decay(self.amp_decay_secs());
        self.amp_env
            .set_attack_ms((self.amp_attack_secs() * 1000.0).max(MIN_AMP_ATTACK_MS));
        self.amp_env.set_hold(self.hold_secs());
        self.amp_env.trigger();
        self.filter_env_time = 0.0;
        self.pitch_env_time = 0.0;
        self.filter_tick = 0;

        // Kept alive for the hit.
        self.source = source;
    }

    fn trigger_hard(&mut self) {
        self.trigger();
        self.amp_env.trigger_hard();
    }

    fn set_texture_pool(&mut self, pool: std::sync::Arc<sample_bank::TexturePool>, lane: usize) {
        self.pool = Some((pool, lane));
    }

    fn process_sample(&mut self) -> f32 {
        self.render(false).0
    }

    /// Two channels when a stereo file plays with the Stereo switch on;
    /// otherwise the right channel is the left, byte for byte.
    fn process_sample_stereo(&mut self) -> (f32, f32) {
        self.render(self.stereo_file())
    }

    fn is_active(&self) -> bool {
        self.active || self.declick.iter().any(|d| d.is_active())
    }

    fn reset(&mut self) {
        self.active = false;
        self.pos = 0.0;
        self.win_end = 0.0;
        self.last_out = [0.0; 2];
        self.pitch_env_time = 1.0e6;
        self.filter.reset();
        self.filter_r.reset();
        self.filter_env_time = 1.0e6;
        self.amp_env.reset();
        self.dc_block.reset();
        self.dc_block_r.reset();
        for d in &mut self.declick {
            d.reset();
        }
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = OneShotSettings::from(settings);

        // Setters only — recreating an envelope resets its state and cuts the
        // sound mid-slider-drag.
        self.amp_env.set_decay(self.amp_decay_secs());
        self.amp_env
            .set_attack_ms((self.amp_attack_secs() * 1000.0).max(MIN_AMP_ATTACK_MS));
        self.amp_env.set_release(self.settings.release.max(0.001));
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
        self.amp_env.set_hold(self.hold_secs());

        self.apply_filter(self.filter_env_value());
        self.sync_saturation();
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.settings.texture = value,
            1 => self.settings.reverse = value,
            2 => self.settings.pitch_fine = value,
            3 => self.settings.pitch_env = value,
            4 => self.settings.pitch_env_attack = value,
            5 => self.settings.pitch_env_hold = value,
            6 => self.settings.pitch_env_decay = value,
            7 => self.settings.pitch_env_atk_curve = value,
            8 => self.settings.pitch_env_dec_curve = value,
            9 => self.settings.filter_type = value,
            10 => self.settings.resonance = value,
            11 => self.settings.filter_attack = value,
            12 => self.settings.filter_hold = value,
            13 => self.settings.filter_atk_curve = value,
            14 => self.settings.filter_dec_curve = value,
            15 => {
                self.settings.saturation_type = value as u8;
                self.saturation.saturation_type =
                    saturation::SaturationType::from(self.settings.saturation_type);
                self.saturation.update_compensation();
            }
            16 => {
                self.settings.saturation_amount = value;
                self.saturation.amount = value;
                self.saturation.update_compensation();
            }
            17 => {
                self.settings.saturation_mix = value;
                self.saturation.mix = value;
                self.saturation.update_compensation();
            }
            18 => {
                self.settings.saturation_output_gain = value;
                self.saturation.output_gain = value;
                self.saturation.update_compensation();
            }
            19 => {
                self.settings.saturation_pre_filter = value;
                self.saturation.pre_filter = value > 0.5;
                self.saturation.update_compensation();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A 0.25 s sine at `hz` as the lane's file, published into a pool.
    fn voice_at(settings: VoiceSettings, hz: f32) -> OneShotVoice {
        let n = 11025;
        let data: Vec<f32> = (0..n)
            .map(|i| (i as f32 * hz * 2.0 * std::f32::consts::PI / 44100.0).sin() * 0.8)
            .collect();
        let peaks = data.iter().map(|s| (*s, *s)).collect();
        let bank = Arc::new(sample_bank::TextureBank {
            source_rate: 44100.0,
            data,
            right: None,
            peaks,
        });
        let pool = Arc::new(sample_bank::TexturePool::new());
        pool.publish(2, Some(bank));
        let mut v = OneShotVoice::new(44100.0, OneShotSettings::from(settings));
        v.set_texture_pool(pool, 2);
        v
    }

    fn voice_with(settings: VoiceSettings) -> OneShotVoice {
        voice_at(settings, 220.0)
    }

    fn render(voice: &mut OneShotVoice, n: usize) -> Vec<f32> {
        voice.trigger();
        (0..n).map(|_| voice.process_sample()).collect()
    }

    fn peak(buf: &[f32]) -> f32 {
        buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
    }

    #[test]
    fn plays_the_file_and_stays_finite() {
        let mut v = voice_with(VoiceSettings::oneshot());
        let out = render(&mut v, 44100);
        assert!(peak(&out) > 0.001, "silent, peak {}", peak(&out));
        assert!(out.iter().all(|s| s.is_finite()), "non-finite sample");
    }

    #[test]
    fn a_lane_without_a_file_stays_inert() {
        let mut v = OneShotVoice::new(44100.0, OneShotSettings::from(VoiceSettings::oneshot()));
        v.trigger();
        let out: Vec<f32> = (0..1000).map(|_| v.process_sample()).collect();
        assert!(peak(&out) < 1e-6, "no file should mean silence");
    }

    #[test]
    fn pitch_ratio_moves_the_playback_rate() {
        let travelled = |semis: f32| {
            let mut s = VoiceSettings::oneshot();
            s.frequency = semis;
            let mut v = voice_with(s);
            v.trigger();
            for _ in 0..4410 {
                v.process_sample();
            }
            v.pos
        };
        let up = travelled(12.0);
        let flat = travelled(0.0);
        assert!(up > flat * 1.3, "an octave up read {up} against {flat}");
    }

    #[test]
    fn pitch_env_sweeps_the_rate_with_ahd() {
        let mut s = VoiceSettings::oneshot();
        s.special[3] = 12.0; // Pitch Env depth (semitones)
        s.special[4] = 0.02; // attack
        s.special[6] = 0.05; // decay
        let mut v = voice_with(s);
        v.trigger();
        // At t=0 the envelope is at the start of its attack: no offset yet.
        assert!(v.pitch_env_value().abs() < 1e-3);
        // Mid-attack it is on its way.
        v.pitch_env_time = 0.01;
        let mid = v.pitch_env_value();
        assert!(mid > 0.05 && mid < 0.95, "attack stuck at {mid}");
        // Well past the decay it is back to zero.
        v.pitch_env_time = 0.5;
        assert!(v.pitch_env_value().abs() < 1e-3, "decay never finishes");
    }

    #[test]
    fn reverse_reads_the_file_backwards() {
        let mut s = VoiceSettings::oneshot();
        s.special[1] = 1.0; // Reverse
        let mut v = voice_with(s);
        v.trigger();
        let start = v.pos;
        for _ in 0..2000 {
            v.process_sample();
        }
        assert!(v.pos < start, "the read went forward: {start} -> {}", v.pos);
        assert!(v.pos >= -2.0, "the read ran past the file start");
    }

    #[test]
    fn forward_and_reverse_differ_and_reverse_sounds() {
        let forward = render(&mut voice_with(VoiceSettings::oneshot()), 8000);
        let mut s = VoiceSettings::oneshot();
        s.special[1] = 1.0;
        let backward = render(&mut voice_with(s), 8000);
        assert!(peak(&backward) > 1e-3, "reversed playback is silent");
        assert_ne!(forward, backward, "Reverse changed nothing");
    }

    /// The filter must behave like the rest of the plugin: `Filter Env` opens
    /// the cutoff toward 20 kHz and it falls back onto its setting.
    #[test]
    fn filter_env_opens_the_cutoff_like_the_other_voices() {
        let mut st = VoiceSettings::oneshot();
        st.filter_freq = 500.0;
        st.filter_env_amount = 1.0;
        let v = voice_with(st);
        let open = v.filter_cutoff(1.0);
        let rest = v.filter_cutoff(0.0);
        assert!((open - 20000.0).abs() < 1.0, "full amount must reach 20 kHz, got {open}");
        assert!((rest - 500.0).abs() < 1.0, "cutoff must rest on its setting, got {rest}");
    }

    #[test]
    fn filter_types_change_the_sound() {
        let rendered = |t: f32| {
            let mut s = VoiceSettings::oneshot();
            s.filter_freq = 1000.0;
            s.special[9] = t; // Filter Type
            render(&mut voice_with(s), 8000)
        };
        let lp = rendered(0.0);
        let hp = rendered(1.0);
        let bp = rendered(2.0);
        assert_ne!(lp, hp, "LP and HP render the same");
        assert_ne!(lp, bp, "LP and BP render the same");
        assert!(peak(&hp) > 1e-4 && peak(&bp) > 1e-4);
    }

    /// The plock path delivers the override through `set_settings` BEFORE
    /// `trigger` (fire_voice_trigger): a plocked cutoff must reach the biquad.
    /// The material is a 4 kHz sine: an LP at 300 Hz must nearly silence it.
    #[test]
    fn filter_freq_delivered_via_set_settings_reaches_the_biquad() {
        let mut dark = voice_at(VoiceSettings::oneshot(), 4000.0);
        let mut st = VoiceSettings::oneshot();
        st.filter_freq = 300.0;
        dark.set_settings(st);
        dark.trigger();
        let dark_out: Vec<f32> = (0..8000).map(|_| dark.process_sample()).collect();

        let mut open = voice_at(VoiceSettings::oneshot(), 4000.0);
        open.trigger();
        let open_out: Vec<f32> = (0..8000).map(|_| open.process_sample()).collect();

        assert!(
            peak(&dark_out) < peak(&open_out) * 0.1,
            "the plocked low-pass did not filter: dark peak {} vs open {}",
            peak(&dark_out),
            peak(&open_out)
        );
    }

    /// A retrigger must not click: the biggest jump at the second trigger must
    /// stay in the range of the ordinary sample-to-sample jumps.
    #[test]
    fn retrigger_is_declicked() {
        let mut v = voice_with(VoiceSettings::oneshot());
        v.trigger();
        let mut out: Vec<f32> = (0..2000).map(|_| v.process_sample()).collect();
        v.trigger();
        out.extend((0..2000).map(|_| v.process_sample()).collect::<Vec<f32>>());
        let jumps: Vec<f32> = out.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        let mut sorted = jumps.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = sorted[sorted.len() / 2];
        let around = jumps[1995..2005].iter().cloned().fold(0.0f32, f32::max);
        assert!(
            around <= median * 3.0 + 1e-3,
            "retrigger jump {around} vs ordinary jump {median}"
        );
    }
}
