//! Zap synthesizer — short laser-like percussion
//!
//! Two algorithms:
//! - 0 Saw Zap: sawtooth oscillator + upward pitch sweep + LP filter
//! - 1 Square Zap: square oscillator + downward pitch sweep + HP filter

use super::{dsp, Voice, VoiceSettings};

const ZAP_ATTACK_MS: f32 = 0.5;

pub struct ZapVoice {
    settings: VoiceSettings,
    sample_rate: f32,

    // Oscillators (stereo pair)
    osc_l: dsp::SawOsc,
    osc_r: dsp::SawOsc,

    // Amplitude envelope
    amp_env: dsp::ExpDecayEnvelope,
    // Filter envelope
    filter_env: dsp::ExpDecayEnvelope,

    // Filter
    filter: dsp::OnePoleFilter,

    // Sweep state
    sweep_phase: f32,     // 0.0 → 1.0 during sweep
    sweep_inc: f32,       // per-sample increment
    sweep_ratio: f32,     // target frequency ratio
    base_freq: f32,

    active: bool,
}

impl ZapVoice {
    pub fn new(sample_rate: f32, settings: VoiceSettings) -> Self {
        let base_freq = settings.frequency.max(20.0);
        let mut osc_l = dsp::SawOsc::new(sample_rate);
        osc_l.set_freq(base_freq);
        let mut osc_r = dsp::SawOsc::new(sample_rate);
        osc_r.set_freq(base_freq);

        let filter_mode = if settings.algo == 1 {
            dsp::FilterMode::HighPass
        } else {
            dsp::FilterMode::LowPass
        };
        let mut filter = dsp::OnePoleFilter::new(filter_mode);
        filter.set_cutoff(settings.filter_freq, sample_rate);

        let amp_env = dsp::ExpDecayEnvelope::new(sample_rate, settings.decay_curve, settings.decay.max(0.005))
            .with_attack_ms(ZAP_ATTACK_MS);

        let filter_env = dsp::ExpDecayEnvelope::new(sample_rate, 8.0, settings.filter_env_decay.max(0.001))
            .with_attack_ms(0.3);

        Self {
            settings,
            sample_rate,
            osc_l,
            osc_r,
            amp_env,
            filter_env,
            filter,
            sweep_phase: 0.0,
            sweep_inc: 0.0,
            sweep_ratio: 1.0,
            base_freq,
            active: false,
        }
    }

    fn update_sweep(&mut self) {
        let sweep_amount = self.settings.special[0].clamp(-1.0, 1.0);
        let speed_ms = self.settings.special[1].clamp(0.5, 50.0);

        // sweep_ratio: -1 → 4.0 (up 2 octaves), +1 → 0.25 (down 2 octaves)
        self.sweep_ratio = 2.0_f32.powf(-sweep_amount * 2.0);
        self.sweep_inc = 1.0 / (speed_ms * 0.001 * self.sample_rate);
    }

    fn current_freq(&self) -> f32 {
        if self.sweep_phase >= 1.0 {
            return self.base_freq * self.sweep_ratio;
        }
        // Linear interpolation from 1.0 to sweep_ratio
        let ratio = 1.0 + (self.sweep_ratio - 1.0) * self.sweep_phase;
        (self.base_freq * ratio).max(20.0)
    }

    fn drive(&self, sample: f32) -> f32 {
        let bite = self.settings.special[2].clamp(0.0, 1.0);
        if bite <= 0.001 {
            return sample;
        }
        // Soft clipping / mild distortion
        let drive_gain = 1.0 + bite * 4.0;
        let s = sample * drive_gain;
        s.tanh()
    }
}

impl Voice for ZapVoice {
    fn trigger(&mut self) {
        self.active = true;
        self.amp_env.trigger();
        self.filter_env.trigger();
        self.sweep_phase = 0.0;
        self.update_sweep();

        // Reset phase for clicky attack
        self.osc_l.phase = 0.0;
        self.osc_r.phase = 0.25; // quadrature for stereo width

        let freq = self.current_freq();
        self.osc_l.set_freq(freq);
        self.osc_r.set_freq(freq);
    }

    fn process_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let amp = self.amp_env.next();
        if amp <= 0.0 && self.sweep_phase >= 1.0 {
            self.active = false;
            return 0.0;
        }

        // Update sweep
        if self.sweep_phase < 1.0 {
            self.sweep_phase += self.sweep_inc;
            if self.sweep_phase > 1.0 {
                self.sweep_phase = 1.0;
            }
            let freq = self.current_freq();
            self.osc_l.set_freq(freq);
            self.osc_r.set_freq(freq);
        }

        let filter_env_val = self.filter_env.next();
        let cutoff = self.settings.filter_freq
            * (1.0 + filter_env_val * self.settings.filter_env_amount * 2.0);
        self.filter.set_cutoff(cutoff.max(50.0), self.sample_rate);

        let raw = self.osc_l.next();
        let filtered = self.filter.process(raw);
        let driven = self.drive(filtered);
        driven * amp * self.settings.volume
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        if self.settings.stereo < 0.5 {
            let m = self.process_sample();
            return (m, m);
        }

        let width = self.settings.special[3].clamp(0.0, 1.0);
        let amp = self.amp_env.next();
        if amp <= 0.0 && self.sweep_phase >= 1.0 {
            self.active = false;
            return (0.0, 0.0);
        }

        // Update sweep
        if self.sweep_phase < 1.0 {
            self.sweep_phase += self.sweep_inc;
            if self.sweep_phase > 1.0 {
                self.sweep_phase = 1.0;
            }
            let freq = self.current_freq();
            self.osc_l.set_freq(freq);
            // Slight detune for right channel based on width
            let detune = 1.0 + width * 0.02;
            self.osc_r.set_freq(freq * detune);
        }

        let filter_env_val = self.filter_env.next();
        let cutoff = self.settings.filter_freq
            * (1.0 + filter_env_val * self.settings.filter_env_amount * 2.0);
        self.filter.set_cutoff(cutoff.max(50.0), self.sample_rate);

        let raw_l = self.osc_l.next();
        let raw_r = self.osc_r.next();
        let filt_l = self.filter.process(raw_l);
        let filt_r = self.filter.process(raw_r);
        let out_l = self.drive(filt_l) * amp * self.settings.volume;
        let out_r = self.drive(filt_r) * amp * self.settings.volume;
        (out_l, out_r)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.trigger(); // reset internal state
        self.amp_env.next();
        self.filter_env.trigger();
        self.filter_env.next();
        self.sweep_phase = 0.0;
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        self.settings = settings;
        self.base_freq = settings.frequency.max(20.0);
        self.osc_l.set_freq(self.base_freq);
        self.osc_r.set_freq(self.base_freq);

        let filter_mode = if settings.algo == 1 {
            dsp::FilterMode::HighPass
        } else {
            dsp::FilterMode::LowPass
        };
        self.filter = dsp::OnePoleFilter::new(filter_mode);
        self.filter.set_cutoff(settings.filter_freq, self.sample_rate);

        self.amp_env = dsp::ExpDecayEnvelope::new(self.sample_rate, settings.decay_curve, settings.decay.max(0.005))
            .with_attack_ms(ZAP_ATTACK_MS);
        self.filter_env = dsp::ExpDecayEnvelope::new(self.sample_rate, 8.0, settings.filter_env_decay.max(0.001))
            .with_attack_ms(0.3);
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
        let filter_mode = if algo == 1 {
            dsp::FilterMode::HighPass
        } else {
            dsp::FilterMode::LowPass
        };
        self.filter = dsp::OnePoleFilter::new(filter_mode);
        self.filter.set_cutoff(self.settings.filter_freq, self.sample_rate);
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        if index < self.settings.special.len() {
            self.settings.special[index] = value;
        }
    }
}
