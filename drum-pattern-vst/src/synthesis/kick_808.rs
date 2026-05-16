//! TR-808 Bass Drum — bridged-T resonator model.
//!
//! Architecture:
//! - Sine oscillator whose frequency is swept by two envelopes:
//!   1. **Snap** (~6 ms attack) — pushes freq up to ~130 Hz for the transient click.
//!   2. **Pitch drop** (slow decay) — drags freq down during the tail for the organic 808 feel.
//! - Amplitude envelope: single-stage exponential decay (50 ms .. 800 ms).
//! - Tone: one-pole LP filter on the output.
//! - Accent: short impulse burst mixed at the attack.

use super::{dsp, Voice, VoiceSettings};

const SNAP_DECAY_SECONDS: f32 = 0.006; // ~6 ms
const SNAP_CURVE: f32 = 8.0;
const PITCH_DROP_CURVE: f32 = 2.0;

pub struct Kick808Voice {
    settings: VoiceSettings,
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
    // Smooths frequency jumps for analog-style pitch slide
    freq_smoother: dsp::OnePoleSmoother,

    active: bool,
}

impl Kick808Voice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
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
            ),
            tone_filter,
            click: dsp::ClickGenerator::new(sample_rate, 15.0, 0.2, 0.05),
            freq_smoother: dsp::OnePoleSmoother::new(sample_rate, 0.5, settings.frequency.max(10.0)),
            active: false,
        }
    }

    /// Pitch-drop time: faster than amplitude decay so the drop is clearly audible.
    fn drop_time(decay: f32) -> f32 {
        (decay * 0.6).max(0.03)
    }

    fn snap_depth_hz(&self) -> f32 {
        // With snap=1.0, push up to ~130 Hz from base.
        let base = self.settings.frequency.max(10.0);
        let target = 130.0f32;
        (target - base).max(0.0) * self.settings.special[1].clamp(0.0, 1.0)
    }

    fn drop_depth_hz(&self) -> f32 {
        // With drop=1.0, drift down by ~35 % of base frequency.
        self.settings.frequency.max(10.0) * 0.35 * self.settings.special[2].clamp(0.0, 1.0)
    }

    fn accent_amount(&self) -> f32 {
        self.settings.special[0].clamp(0.0, 1.0)
    }

    fn update_derived_params(&mut self) {
        self.amp_env.set_decay(self.settings.decay.max(0.05));
        self.amp_env.set_release(self.settings.release.max(0.001));
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
        self.drop_env.set_decay(Self::drop_time(self.settings.decay));
        self.tone_filter
            .set_cutoff(self.settings.filter_freq, self.sample_rate);
        self.freq_smoother.set_time_ms(self.sample_rate, 0.5);
    }
}

impl Voice for Kick808Voice {
    fn trigger(&mut self) {
        self.active = true;
        let base = self.settings.frequency.max(10.0);
        if self.settings.analog < 0.5 {
            // Digital stable: reset phase and smoother for identical sound every hit
            self.osc.phase = 0.0;
            self.freq_smoother.reset(base);
        }
        self.osc.set_freq(base);
        self.snap_env.trigger();
        self.drop_env.trigger();
        self.amp_env.trigger();
        if self.accent_amount() > 0.0 {
            self.click.trigger();
        }
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let base = self.settings.frequency.max(10.0);
        let snap = self.snap_env.next();
        let drop = self.drop_env.next();

        // Frequency modulation: base + snap_peak*env - drop_depth*env
        let target_freq = (base + self.snap_depth_hz() * snap - self.drop_depth_hz() * (1.0 - drop))
            .max(10.0);
        let freq = self.freq_smoother.process(target_freq);
        self.osc.set_freq(freq);

        let raw = self.osc.next();
        let env = self.amp_env.next();
        if env <= 0.0 {
            self.active = false;
            return 0.0;
        }

        let body = self.tone_filter.process(raw) * env * self.settings.volume;

        let click = if self.accent_amount() > 0.0 && self.click.is_active() {
            self.click.next() * self.accent_amount()
        } else {
            0.0
        };

        body + click
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
        self.settings = settings;
        self.update_derived_params();
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index < self.settings.special.len() {
            self.settings.special[index] = value;
        }
    }
}
