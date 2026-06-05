//! TR-808 Bass Drum — bridged-T resonator model.
//!
//! Architecture:
//! - Sine oscillator whose frequency is swept by two envelopes:
//!   1. **Snap** (~6 ms attack) — pushes freq up to ~130 Hz for the transient click.
//!   2. **Pitch drop** (slow decay) — drags freq down during the tail for the organic 808 feel.
//! - Amplitude envelope: single-stage exponential decay (50 ms .. 800 ms).
//! - Tone: one-pole LP filter on the output.
//! - Accent: short impulse burst mixed at the attack.

use super::{dsp, saturation, settings::kick_808::Kick808Settings, Voice, VoiceSettings};

const SNAP_DECAY_SECONDS: f32 = 0.006; // ~6 ms
const SNAP_CURVE: f32 = 8.0;
const PITCH_DROP_CURVE: f32 = 2.0;

pub struct Kick808Voice {
    settings: Kick808Settings,
    sample_rate: f32,

    // Core oscillator
    osc: dsp::SineOsc,

    // Snap envelope: fast pitch rise at attack
    snap_env: dsp::ExpDecayEnvelope,
    // Pitch-drop envelope: slow drift toward grave during decay
    drop_env: dsp::ExpDecayEnvelope,
    // Body amplitude envelope (decay + release)
    amp_env: dsp::DecayReleaseEnvelope,
    // Output tone filter (passive LP)
    tone_filter: dsp::OnePoleFilter,
    // Accent click
    click: dsp::ClickGenerator,
    // LP filter on the click to tame high-freq noise
    click_filter: dsp::OnePoleFilter,
    // Smooths frequency jumps for analog-style pitch slide
    freq_smoother: dsp::OnePoleSmoother,
    // DC blocker to kill offset clicks on asymmetric retriggers
    dc_blocker: dsp::DcBlocker,
    // Saturation stage
    saturation: saturation::SaturationConfig,
    /// Per-hit analog drift (breathing) — pitch/level/time variation per hit.
    drift: dsp::AnalogDrift,

    active: bool,
}

impl Kick808Voice {
    pub fn new(sample_rate: f32, settings: Kick808Settings) -> Self {
        let mut osc = dsp::SineOsc::new(sample_rate);
        osc.set_freq(settings.frequency.max(10.0));

        let mut tone_filter = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        tone_filter.set_cutoff(settings.filter_freq, sample_rate);

        Self {
            settings,
            sample_rate,
            osc,
            snap_env: dsp::ExpDecayEnvelope::new(sample_rate, SNAP_CURVE, SNAP_DECAY_SECONDS),
            drop_env: dsp::ExpDecayEnvelope::new(
                sample_rate,
                PITCH_DROP_CURVE,
                Self::drop_time(settings.decay),
            ),
            amp_env: dsp::DecayReleaseEnvelope::new(
                sample_rate,
                settings.decay_curve,
                settings.decay.max(0.05),
                settings.release_curve,
                settings.release.max(0.001),
            )
            .with_attack_ms(settings.attack * 1000.0),
            tone_filter,
            click: dsp::ClickGenerator::new(sample_rate, 15.0, 0.2, 2.0),
            click_filter: dsp::OnePoleFilter::new(dsp::FilterMode::LowPass),
            freq_smoother: dsp::OnePoleSmoother::new(
                sample_rate,
                5.0,
                settings.frequency.max(10.0),
            ),
            dc_blocker: dsp::DcBlocker::default(),
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::None,
                amount: 0.0,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
            },
            drift: dsp::AnalogDrift::new(0x8080_8080),
            active: false,
        }
    }

    /// Pitch-drop time: faster than amplitude decay so the drop is clearly audible.
    fn drop_time(decay: f32) -> f32 {
        (decay * 0.6).max(0.03)
    }

    fn snap_depth_hz(&self) -> f32 {
        // EXTREME for testing: push up to ~800 Hz from base.
        let base = self.settings.frequency.max(10.0);
        let target = 800.0f32;
        (target - base).max(0.0) * self.settings.snap.clamp(0.0, 1.0)
    }

    fn drop_depth_hz(&self) -> f32 {
        // EXTREME for testing: drift down by ~150 % of base frequency.
        self.settings.frequency.max(10.0) * 1.5 * self.settings.pitch_drop.clamp(0.0, 1.0)
    }

    fn accent_amount(&self) -> f32 {
        self.settings.accent.clamp(0.0, 1.0)
    }

    fn update_derived_params(&mut self) {
        self.amp_env.set_decay(self.settings.decay.max(0.05));
        self.amp_env.set_attack_ms(self.settings.attack * 1000.0);
        self.amp_env.set_release(self.settings.release.max(0.001));
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
        self.drop_env
            .set_decay(Self::drop_time(self.settings.decay));
        self.tone_filter
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        // Click filter: dedicated LP cutoff from special[3].
        self.click_filter.set_cutoff(
            self.settings.click_tone.clamp(100.0, 8000.0),
            self.sample_rate,
        );
        self.freq_smoother.set_time_ms(self.sample_rate, 5.0);
    }
}

impl Voice for Kick808Voice {
    fn trigger(&mut self) {
        let is_cold_start = !self.active;
        self.active = true;
        // analog = per-hit drift (breathing) ; digital = bit-identical hits.
        self.drift.trigger(self.settings.analog >= 0.5);
        let base = self.settings.frequency.max(10.0);
        if self.settings.analog < 0.5 && is_cold_start {
            // Digital stable: reset phase and smoother only on cold start.
            // Never reset during a retrigger on a ringing tail — that causes a click.
            self.osc.phase = 0.0;
            self.freq_smoother.reset(base);
        }
        self.osc.set_freq(base);
        // Per-hit envelope-time drift: scale decay/release so the tail length varies.
        self.amp_env.set_decay(self.settings.decay * self.drift.time);
        self.amp_env.set_release(self.settings.release * self.drift.time);
        self.snap_env.trigger();
        self.drop_env.trigger();
        self.amp_env.trigger();
        if self.accent_amount() > 0.0 {
            self.click.trigger();
        }
    }

    fn trigger_hard(&mut self) {
        self.active = true;
        self.amp_env.trigger_hard();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let base = self.settings.frequency.max(10.0);
        let snap = self.snap_env.next();
        let drop = self.drop_env.next();

        // Frequency modulation: base + snap_peak*env - drop_depth*env
        let target_freq = (base + self.snap_depth_hz() * snap
            - self.drop_depth_hz() * (1.0 - drop))
            .max(10.0)
            * self.drift.pitch;
        let freq = self.freq_smoother.process(target_freq);
        self.osc.set_freq(freq);

        let raw = self.osc.next();
        let env = self.amp_env.next();
        if env <= 0.0 {
            self.active = false;
            return 0.0;
        }

        let body = self.tone_filter.process(raw)
            * env
            * self.settings.volume
            * self.drift.level;

        let click = if self.accent_amount() > 0.0 && self.click.is_active() {
            self.click_filter.process(self.click.next()) * self.accent_amount()
        } else {
            0.0
        };

        let out = self.dc_blocker.process(body + click);
        self.saturation.process(out)
    }

    fn is_active(&self) -> bool {
        self.active || self.click.is_active()
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
        self.snap_env.reset();
        self.drop_env.reset();
        self.click.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = Kick808Settings::from(settings);
        self.update_derived_params();
        self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }
    
    fn set_special_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.settings.accent = value,
            1 => self.settings.snap = value,
            2 => self.settings.pitch_drop = value,
            3 => self.settings.click_tone = value,
            4 => {
                self.settings.saturation_type = value as u8;
                self.saturation.saturation_type = saturation::SaturationType::from(self.settings.saturation_type);
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
            _ => {}
        }
    }
}
