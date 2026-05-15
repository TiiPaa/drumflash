//! TR-606 snare drum — separate voice modelling the bridged-T resonator
//! topology of the original Roland TR-606.
//!
//! Signal chain (matches the user's spec):
//! ```text
//!   White noise ──► passive LP (~3 kHz "softener") ──┬──► × env (Swing VCA) ──► Biquad bandpass (bridged-T) ──┐
//!                                                    │                                                          ├──► mix → out
//!                                                    └────────────────────► × env ──────────────────────────────┘
//!                                                                          (dry wires / rattle layer)
//! ```
//!
//! All filter / resonator parameters are exposed:
//! - `settings.frequency`       → bandpass centre (~150–400 Hz typical)
//! - `settings.filter_freq`     → LP softener cutoff (Hz)
//! - `settings.special[0]`      → resonance Q (0.5 .. 12)
//! - `settings.special[1]`      → tone mix: 0 = mostly wires, 1 = mostly body
//! - `settings.special[2]`      → wire crispness: HP gain of the dry layer

use super::{dsp, Voice, VoiceSettings};

const ATTACK_MS: f32 = 1.5;

pub struct Snare606Voice {
    settings: VoiceSettings,
    sample_rate: f32,

    noise: dsp::WhiteNoise,
    lp_softener: dsp::OnePoleFilter,
    /// Highpass on the dry wires layer — keeps it crisp on top of the body.
    wires_hp: dsp::OnePoleFilter,
    resonator: dsp::Biquad,
    envelope: dsp::DecayReleaseEnvelope,

    active: bool,
}

impl Snare606Voice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let mut lp_softener = dsp::OnePoleFilter::new(dsp::FilterMode::LowPass);
        lp_softener.set_cutoff(settings.filter_freq.max(500.0), sample_rate);

        let mut wires_hp = dsp::OnePoleFilter::new(dsp::FilterMode::HighPass);
        wires_hp.set_cutoff(1500.0, sample_rate);

        let mut resonator = dsp::Biquad::new();
        let q = settings.special[0].clamp(0.5, 12.0);
        resonator.set_bandpass(settings.frequency.max(80.0), q, sample_rate);

        let mut envelope = dsp::DecayReleaseEnvelope::new(
            sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms(ATTACK_MS);
        envelope.set_hold(settings.hold);

        Self {
            settings,
            sample_rate,
            noise: dsp::WhiteNoise::new(0x5A5A_5A5A),
            lp_softener,
            wires_hp,
            resonator,
            envelope,
            active: false,
        }
    }

    fn resonance_q(&self) -> f32 {
        self.settings.special[0].clamp(0.5, 12.0)
    }

    fn tone_mix(&self) -> f32 {
        self.settings.special[1].clamp(0.0, 1.0)
    }

    fn wire_crisp(&self) -> f32 {
        self.settings.special[2].clamp(0.0, 1.0)
    }
}

impl Voice for Snare606Voice {
    fn trigger(&mut self) {
        self.active = true;
        // Keep noise generator, filter states and resonator state continuous —
        // analog-style retrigger (matches kick/tom convention in this codebase).
        self.envelope.trigger();
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let env = self.envelope.next();
        if !self.envelope.is_active() {
            self.active = false;
            return 0.0;
        }

        // Stage 1: white noise. Stage 2: passive LP "softener".
        let raw = self.noise.next();
        let softened = self.lp_softener.process(raw);

        // Stage 3: Swing-VCA — envelope-shaped excitation.
        let excitation = softened * env;

        // Stage 4: bridged-T resonator (Biquad bandpass) — the drum head's
        // pseudo-tonal body. Excited by enveloped noise, it gives the kick of
        // a real snare without needing a separate body oscillator.
        let body = self.resonator.process(excitation);

        // Stage 5: dry wires layer — softened noise filtered through a HP
        // so it sits on top of the body without muddying the lows.
        let wires_raw = self.wires_hp.process(softened) * env;

        // Mix: tone_mix blends body vs wires; wire_crisp boosts wires HP component.
        let tone = self.tone_mix();
        let crisp = self.wire_crisp();
        let body_gain = 0.4 + tone * 0.6; // 0.4 .. 1.0
        let wires_gain = (1.0 - tone) * 0.5 + crisp * 0.4;

        (body * body_gain + wires_raw * wires_gain) * self.settings.volume
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.envelope.reset();
        self.lp_softener.reset();
        self.wires_hp.reset();
        self.resonator.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        let q_changed = (settings.special[0] - self.settings.special[0]).abs() > 1e-4;
        let freq_changed = (settings.frequency - self.settings.frequency).abs() > 1e-3;
        let lp_changed = (settings.filter_freq - self.settings.filter_freq).abs() > 1e-3;

        self.settings = settings;

        if lp_changed {
            self.lp_softener
                .set_cutoff(settings.filter_freq.max(500.0), self.sample_rate);
        }
        if q_changed || freq_changed {
            let q = settings.special[0].clamp(0.5, 12.0);
            self.resonator
                .set_bandpass(settings.frequency.max(80.0), q, self.sample_rate);
        }
        self.envelope = dsp::DecayReleaseEnvelope::new(
            self.sample_rate,
            settings.decay_curve,
            settings.decay,
            settings.release_curve,
            settings.release,
        )
        .with_attack_ms(ATTACK_MS);
        self.envelope.set_hold(settings.hold);
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index < self.settings.special.len() {
            self.settings.special[index] = value;
            // Q lives in special[0] — re-tune the resonator immediately so
            // moving the slider is audible without waiting for set_settings.
            if index == 0 {
                let q = self.resonance_q();
                self.resonator
                    .set_bandpass(self.settings.frequency.max(80.0), q, self.sample_rate);
            }
        }
    }

}
