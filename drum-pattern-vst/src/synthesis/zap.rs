//! Zap synthesizer — laser/blaster percussion
//!
//! Architecture inspired by the Sequential Pro-One:
//! - Two oscillators (A + B) with FM for metallic character
//! - Exponential pitch sweep via dedicated pitch envelope
//! - Very short amplitude envelope (attack ~0, decay 50–300 ms)
//! - Slap delay (80–150 ms) for spatial context
//!
//! Two algorithms:
//! - 0 Sine: clean "pew-pew" cartoon laser
//! - 1 Saw: aggressive sci-fi blaster

use super::{dsp, Voice, VoiceSettings};

const MAX_DELAY_SAMPLES: usize = 8192;
const SLAP_DELAY_MS: f32 = 100.0;
const DELAY_FEEDBACK: f32 = 0.35;

enum ZapOsc {
    Sine(dsp::SineOsc),
    Saw(dsp::SawOsc),
}

impl ZapOsc {
    fn new(sample_rate: f32, algo: u8) -> Self {
        if algo == 0 {
            ZapOsc::Sine(dsp::SineOsc::new(sample_rate))
        } else {
            ZapOsc::Saw(dsp::SawOsc::new(sample_rate))
        }
    }

    fn set_freq(&mut self, freq: f32) {
        match self {
            ZapOsc::Sine(o) => o.set_freq(freq),
            ZapOsc::Saw(o) => o.set_freq(freq),
        }
    }

    fn next(&mut self) -> f32 {
        match self {
            ZapOsc::Sine(o) => o.next(),
            ZapOsc::Saw(o) => o.next(),
        }
    }

    fn reset_phase(&mut self, phase: f32) {
        match self {
            ZapOsc::Sine(o) => o.phase = phase,
            ZapOsc::Saw(o) => o.phase = phase,
        }
    }
}

pub struct ZapVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    // Carrier oscillators (left / right)
    osc_a_l: ZapOsc,
    osc_a_r: ZapOsc,
    // Modulator oscillators (FM source)
    osc_b_l: ZapOsc,
    osc_b_r: ZapOsc,

    // Pitch sweep envelope — exponential curve
    sweep_env: dsp::PitchEnvelope,

    // Amplitude envelope — very short, no sustain
    amp_env: dsp::ExpDecayEnvelope,

    // Slap delay buffers
    delay_buf_l: [f32; MAX_DELAY_SAMPLES],
    delay_buf_r: [f32; MAX_DELAY_SAMPLES],
    delay_pos: usize,
    delay_samples: usize,

    active: bool,
}

impl ZapVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let algo = settings.algo;
        let base_freq = settings.frequency.max(20.0);

        let mut osc_a_l = ZapOsc::new(sample_rate, algo);
        let mut osc_a_r = ZapOsc::new(sample_rate, algo);
        let mut osc_b_l = ZapOsc::new(sample_rate, algo);
        let mut osc_b_r = ZapOsc::new(sample_rate, algo);
        osc_a_l.set_freq(base_freq);
        osc_a_r.set_freq(base_freq);
        osc_b_l.set_freq(base_freq * 1.5);
        osc_b_r.set_freq(base_freq * 1.5);

        let (sweep_start, sweep_end) = Self::sweep_ratios(&settings);
        let sweep_time = Self::sweep_time(&settings);
        let sweep_env = dsp::PitchEnvelope::new(sample_rate, sweep_start, sweep_end, sweep_time);

        let decay = settings.decay.max(0.01).min(0.3);
        let amp_env = dsp::ExpDecayEnvelope::new(sample_rate, settings.decay_curve, decay)
            .with_attack_ms(0.5);

        let delay_samples = ((SLAP_DELAY_MS * 0.001) * sample_rate).round() as usize;
        let delay_samples = delay_samples.min(MAX_DELAY_SAMPLES - 1);

        Self {
            settings,
            sample_rate,
            osc_a_l,
            osc_a_r,
            osc_b_l,
            osc_b_r,
            sweep_env,
            amp_env,
            delay_buf_l: [0.0; MAX_DELAY_SAMPLES],
            delay_buf_r: [0.0; MAX_DELAY_SAMPLES],
            delay_pos: 0,
            delay_samples,
            active: false,
        }
    }

    fn sweep_ratios(settings: &VoiceSettings) -> (f32, f32) {
        let amount = settings.special[0].clamp(-1.0, 1.0);
        if amount >= 0.0 {
            // Descending laser (high → low)
            let depth = 0.05_f32.powf(amount); // 1.0 → 0.05
            (1.0, depth)
        } else {
            // Ascending power-up (low → high)
            let depth = 0.05_f32.powf(-amount); // 0.05 → 1.0
            (depth, 1.0)
        }
    }

    fn sweep_time(settings: &VoiceSettings) -> f32 {
        let speed_ms = settings.special[1].clamp(1.0, 300.0);
        speed_ms / 1000.0
    }

    fn rebuild_sweep(&mut self) {
        let (start, end) = Self::sweep_ratios(&self.settings);
        let time = Self::sweep_time(&self.settings);
        self.sweep_env = dsp::PitchEnvelope::new(self.sample_rate, start, end, time);
    }
}

impl Voice for ZapVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.rebuild_sweep();
        self.sweep_env.trigger();
        self.amp_env.trigger();

        // Reset phases for crisp attack
        self.osc_a_l.reset_phase(0.0);
        self.osc_a_r.reset_phase(0.25);
        self.osc_b_l.reset_phase(0.5);
        self.osc_b_r.reset_phase(0.75);
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let amp = self.amp_env.next();
        if amp <= 0.0 && !self.sweep_env.is_active() {
            self.active = false;
            return 0.0;
        }

        let base = self.settings.frequency.max(20.0);
        let ratio = self.sweep_env.next();
        let freq = base * ratio;

        let bite = self.settings.special[2].clamp(0.0, 1.0);
        let fm_deviation = bite * 3000.0;

        // FM: osc B modulates osc A frequency
        self.osc_b_l.set_freq(freq * 1.5);
        let mod_sample = self.osc_b_l.next();
        self.osc_a_l.set_freq(freq + mod_sample * fm_deviation);
        let dry = self.osc_a_l.next() * amp * self.settings.volume;

        // Simple mono delay
        let wet = self.delay_buf_l[self.delay_pos];
        self.delay_buf_l[self.delay_pos] = dry + wet * DELAY_FEEDBACK;
        self.delay_pos += 1;
        if self.delay_pos >= self.delay_samples {
            self.delay_pos = 0;
        }

        let width = self.settings.special[3].clamp(0.0, 1.0);
        dry + wet * width * 0.5
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }

        let amp = self.amp_env.next();
        if amp <= 0.0 && !self.sweep_env.is_active() {
            self.active = false;
            return (0.0, 0.0);
        }

        let base = self.settings.frequency.max(20.0);
        let ratio = self.sweep_env.next();
        let freq = base * ratio;

        let bite = self.settings.special[2].clamp(0.0, 1.0);
        let fm_deviation = bite * 3000.0;
        let width = self.settings.special[3].clamp(0.0, 1.0);

        // Detune right channel slightly for stereo width
        let detune = 1.0 + width * 0.008;

        // Left channel
        self.osc_b_l.set_freq(freq * 1.5);
        let mod_l = self.osc_b_l.next();
        self.osc_a_l.set_freq(freq + mod_l * fm_deviation);
        let dry_l = self.osc_a_l.next() * amp * self.settings.volume;

        // Right channel
        self.osc_b_r.set_freq(freq * 1.5 * detune);
        let mod_r = self.osc_b_r.next();
        self.osc_a_r.set_freq(freq * detune + mod_r * fm_deviation);
        let dry_r = self.osc_a_r.next() * amp * self.settings.volume;

        // Slap delay
        let wet_l = self.delay_buf_l[self.delay_pos];
        let wet_r = self.delay_buf_r[self.delay_pos];
        self.delay_buf_l[self.delay_pos] = dry_l + wet_l * DELAY_FEEDBACK;
        self.delay_buf_r[self.delay_pos] = dry_r + wet_r * DELAY_FEEDBACK;
        self.delay_pos += 1;
        if self.delay_pos >= self.delay_samples {
            self.delay_pos = 0;
        }

        let delay_mix = width * 0.5;
        (dry_l + wet_l * delay_mix, dry_r + wet_r * delay_mix)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
        self.sweep_env.trigger(); // reset internal state
        self.delay_buf_l = [0.0; MAX_DELAY_SAMPLES];
        self.delay_buf_r = [0.0; MAX_DELAY_SAMPLES];
        self.delay_pos = 0;
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        let algo_changed = self.settings.algo != settings.algo;
        self.settings = settings;
        let base_freq = settings.frequency.max(20.0);

        if algo_changed {
            let algo = settings.algo;
            self.osc_a_l = ZapOsc::new(self.sample_rate, algo);
            self.osc_a_r = ZapOsc::new(self.sample_rate, algo);
            self.osc_b_l = ZapOsc::new(self.sample_rate, algo);
            self.osc_b_r = ZapOsc::new(self.sample_rate, algo);
        }
        self.osc_a_l.set_freq(base_freq);
        self.osc_a_r.set_freq(base_freq);
        self.osc_b_l.set_freq(base_freq * 1.5);
        self.osc_b_r.set_freq(base_freq * 1.5);

        self.rebuild_sweep();

        let decay = settings.decay.max(0.01).min(0.3);
        self.amp_env = dsp::ExpDecayEnvelope::new(self.sample_rate, settings.decay_curve, decay)
            .with_attack_ms(0.5);
    }

    fn set_algo(&mut self, algo: u8) {
        if self.settings.algo == algo {
            return;
        }
        self.settings.algo = algo;
        let base_freq = self.settings.frequency.max(20.0);
        self.osc_a_l = ZapOsc::new(self.sample_rate, algo);
        self.osc_a_r = ZapOsc::new(self.sample_rate, algo);
        self.osc_b_l = ZapOsc::new(self.sample_rate, algo);
        self.osc_b_r = ZapOsc::new(self.sample_rate, algo);
        self.osc_a_l.set_freq(base_freq);
        self.osc_a_r.set_freq(base_freq);
        self.osc_b_l.set_freq(base_freq * 1.5);
        self.osc_b_r.set_freq(base_freq * 1.5);
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index < self.settings.special.len() {
            self.settings.special[index] = value;
        }
    }
}
