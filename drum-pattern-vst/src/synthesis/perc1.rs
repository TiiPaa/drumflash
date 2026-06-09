//! Perc1 synthesizer — laser/blaster percussion
//!
//! Architecture inspired by the Sequential Pro-One:
//! - Two oscillators (A + B) with FM for metallic character
//! - Exponential pitch sweep via dedicated pitch envelope
//! - Bi-stage amplitude envelope (decay + release) with adjustable release tail
//! - Filter + filter envelope for tonal shaping
//! - Slap delay (80–150 ms) for spatial context
//!
//! Two algorithms:
//! - 0 Sine: clean "pew-pew" cartoon laser
//! - 1 Saw: aggressive sci-fi blaster

use super::{dsp, saturation, settings::perc1::Perc1Settings, Voice, VoiceSettings};

const MAX_DELAY_SAMPLES: usize = 8192;
const SLAP_DELAY_MS: f32 = 100.0;
const DELAY_FEEDBACK: f32 = 0.35;
/// Anti-click floor for the amplitude attack (a true 0 ms attack is a step = click).
const MIN_AMP_ATTACK_MS: f32 = 0.3;

enum Perc1Osc {
    Sine(dsp::SineOsc),
    Saw(dsp::SawOsc),
}

impl Perc1Osc {
    fn new(sample_rate: f32, algo: u8) -> Self {
        if algo == 0 {
            Perc1Osc::Sine(dsp::SineOsc::new(sample_rate))
        } else {
            Perc1Osc::Saw(dsp::SawOsc::new(sample_rate))
        }
    }

    fn set_freq(&mut self, freq: f32) {
        match self {
            Perc1Osc::Sine(o) => o.set_freq(freq),
            Perc1Osc::Saw(o) => o.set_freq(freq),
        }
    }

    fn next(&mut self) -> f32 {
        match self {
            Perc1Osc::Sine(o) => o.next(),
            Perc1Osc::Saw(o) => o.next(),
        }
    }

    fn reset_phase(&mut self, phase: f32) {
        match self {
            Perc1Osc::Sine(o) => o.phase = phase,
            Perc1Osc::Saw(o) => o.phase = phase,
        }
    }
}

pub struct Perc1Voice {
    settings: Perc1Settings,
    sample_rate: f32,

    // Carrier oscillators (left / right)
    osc_a_l: Perc1Osc,
    osc_a_r: Perc1Osc,
    // Modulator oscillators (FM source)
    osc_b_l: Perc1Osc,
    osc_b_r: Perc1Osc,

    // Pitch sweep envelope — exponential curve
    sweep_env: dsp::PitchEnvelope,

    // Amplitude envelope — bi-stage decay + release
    amp_env: dsp::DecayReleaseEnvelope,

    // Filter + filter envelope
    filter: dsp::OnePoleFilter,
    filter_env: dsp::ExpDecayEnvelope,

    // Slap delay buffers
    delay_buf_l: [f32; MAX_DELAY_SAMPLES],
    delay_buf_r: [f32; MAX_DELAY_SAMPLES],
    delay_pos: usize,
    delay_samples: usize,
    // Saturation stage
    saturation: saturation::SaturationConfig,
    // DC blockers (per channel) — clean the asymmetric drift from FM + saturation.
    dc_block_l: dsp::DcBlocker,
    dc_block_r: dsp::DcBlocker,
    /// Per-hit analog drift (breathing) — pitch/level/time variation per hit.
    drift: dsp::AnalogDrift,

    active: bool,
}

impl Perc1Voice {
    pub fn new(sample_rate: f32, settings: Perc1Settings) -> Self {
        let algo = settings.algo;
        let base_freq = settings.frequency.max(20.0);

        let mut osc_a_l = Perc1Osc::new(sample_rate, algo);
        let mut osc_a_r = Perc1Osc::new(sample_rate, algo);
        let mut osc_b_l = Perc1Osc::new(sample_rate, algo);
        let mut osc_b_r = Perc1Osc::new(sample_rate, algo);
        osc_a_l.set_freq(base_freq);
        osc_a_r.set_freq(base_freq);
        osc_b_l.set_freq(base_freq * 1.5);
        osc_b_r.set_freq(base_freq * 1.5);

        let (sweep_start, sweep_end) = Self::sweep_ratios(&settings);
        let sweep_time = Self::sweep_time(&settings);
        let sweep_env = dsp::PitchEnvelope::new(sample_rate, sweep_start, sweep_end, sweep_time);

        let decay = settings.decay.max(0.01).min(2.0);
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
        let filter_env_decay = settings.filter_env_decay.max(0.01).min(2.0);
        let filter_env =
            dsp::ExpDecayEnvelope::new(sample_rate, settings.decay_curve, filter_env_decay)
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
            filter,
            filter_env,
            delay_buf_l: [0.0; MAX_DELAY_SAMPLES],
            delay_buf_r: [0.0; MAX_DELAY_SAMPLES],
            delay_pos: 0,
            delay_samples,
            saturation: saturation::SaturationConfig {
                saturation_type: saturation::SaturationType::None,
                amount: 0.0,
                mix: 1.0,
                output_gain: 1.0,
                pre_filter: false,
            },
            dc_block_l: dsp::DcBlocker::default(),
            dc_block_r: dsp::DcBlocker::default(),
            drift: dsp::AnalogDrift::new(0x1111_2222),
            active: false,
        }
    }

    fn sweep_ratios(settings: &Perc1Settings) -> (f32, f32) {
        let amount = settings.sweep.clamp(-1.0, 1.0);
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

    fn sweep_time(settings: &Perc1Settings) -> f32 {
        let speed_ms = settings.speed.clamp(1.0, 300.0);
        speed_ms / 1000.0
    }

    fn rebuild_sweep(&mut self) {
        let (start, end) = Self::sweep_ratios(&self.settings);
        let time = Self::sweep_time(&self.settings);
        self.sweep_env = dsp::PitchEnvelope::new(self.sample_rate, start, end, time);
    }
}

impl Voice for Perc1Voice {
    fn trigger(&mut self) {
        let was_active = self.active;
        self.active = true;
        // analog = per-hit drift (breathing) ; digital = bit-identical hits.
        self.drift.trigger(self.settings.analog >= 0.5);
        self.amp_env
            .set_decay(self.settings.decay * self.drift.time);
        self.amp_env
            .set_release(self.settings.release * self.drift.time);
        self.rebuild_sweep();
        self.sweep_env.trigger();
        self.amp_env.trigger();
        self.filter_env.trigger();

        // Cold start only (voice was silent): reset oscillator phases for a crisp,
        // consistent attack + the L/R stereo phase spread. On a retrigger during a
        // ringing tail we must NOT reset phase — that jump is the click parasite.
        if !was_active {
            self.osc_a_l.reset_phase(0.0);
            self.osc_a_r.reset_phase(0.25);
            self.osc_b_l.reset_phase(0.5);
            self.osc_b_r.reset_phase(0.75);
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

        let amp = self.amp_env.next();
        if amp <= 0.0 && !self.sweep_env.is_active() {
            self.active = false;
            return 0.0;
        }

        let base = self.settings.frequency.max(20.0);
        let ratio = self.sweep_env.next();
        let freq = base * ratio * self.drift.pitch;

        let bite = self.settings.bite.clamp(0.0, 1.0);
        let fm_deviation = bite * 3000.0;

        // FM: osc B modulates osc A frequency
        self.osc_b_l.set_freq(freq * 1.5);
        let mod_sample = self.osc_b_l.next();
        self.osc_a_l.set_freq(freq + mod_sample * fm_deviation);
        let mut dry = self.osc_a_l.next() * amp * self.settings.volume * self.drift.level;

        // Filter — additive envelope: Cutoff at rest + (envelope × amount × depth)
        let filter_env_val = self.filter_env.next();
        let filter_freq = self.settings.filter_freq.max(20.0).min(20000.0);
        let filter_env_amount = self.settings.filter_env_amount;
        let effective_freq = filter_freq + filter_env_val * filter_env_amount * 15000.0;
        self.filter
            .set_cutoff(effective_freq.max(20.0).min(20000.0), self.sample_rate);
        dry = self.filter.process(dry);

        // Simple mono delay
        let wet = self.delay_buf_l[self.delay_pos];
        self.delay_buf_l[self.delay_pos] = dry + wet * DELAY_FEEDBACK;
        self.delay_pos += 1;
        if self.delay_pos >= self.delay_samples {
            self.delay_pos = 0;
        }

        let width = self.settings.width.clamp(0.0, 1.0);
        self.dc_block_l
            .process(self.saturation.process(dry + wet * width * 0.5))
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        if !self.active {
            return (0.0, 0.0);
        }
        if self.settings.stereo < 0.5 {
            let m = self.process_sample();
            return (m, m);
        }

        let amp = self.amp_env.next();
        if amp <= 0.0 && !self.sweep_env.is_active() {
            self.active = false;
            return (0.0, 0.0);
        }

        let base = self.settings.frequency.max(20.0);
        let ratio = self.sweep_env.next();
        let freq = base * ratio * self.drift.pitch;

        let bite = self.settings.bite.clamp(0.0, 1.0);
        let fm_deviation = bite * 3000.0;
        let width = self.settings.width.clamp(0.0, 1.0);

        // Detune right channel slightly for stereo width
        let detune = 1.0 + width * 0.008;

        // Left channel
        self.osc_b_l.set_freq(freq * 1.5);
        let mod_l = self.osc_b_l.next();
        self.osc_a_l.set_freq(freq + mod_l * fm_deviation);
        let mut dry_l = self.osc_a_l.next() * amp * self.settings.volume * self.drift.level;

        // Right channel
        self.osc_b_r.set_freq(freq * 1.5 * detune);
        let mod_r = self.osc_b_r.next();
        self.osc_a_r.set_freq(freq * detune + mod_r * fm_deviation);
        let mut dry_r = self.osc_a_r.next() * amp * self.settings.volume * self.drift.level;

        // Filter — additive envelope
        let filter_env_val = self.filter_env.next();
        let filter_freq = self.settings.filter_freq.max(20.0).min(20000.0);
        let filter_env_amount = self.settings.filter_env_amount;
        let effective_freq = filter_freq + filter_env_val * filter_env_amount * 15000.0;
        self.filter
            .set_cutoff(effective_freq.max(20.0).min(20000.0), self.sample_rate);
        dry_l = self.filter.process(dry_l);
        dry_r = self.filter.process(dry_r);

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
        let l = self
            .dc_block_l
            .process(self.saturation.process(dry_l + wet_l * delay_mix));
        let r = self
            .dc_block_r
            .process(self.saturation.process(dry_r + wet_r * delay_mix));
        (l, r)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn reset(&mut self) {
        self.active = false;
        self.amp_env.reset();
        self.filter_env.reset();
        self.sweep_env.trigger(); // reset internal state
        self.delay_buf_l = [0.0; MAX_DELAY_SAMPLES];
        self.delay_buf_r = [0.0; MAX_DELAY_SAMPLES];
        self.delay_pos = 0;
        self.dc_block_l.reset();
        self.dc_block_r.reset();
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        let new = Perc1Settings::from(settings);
        let algo_changed = self.settings.algo != new.algo;
        self.settings = new;
        let base_freq = self.settings.frequency.max(20.0);

        if algo_changed {
            let algo = self.settings.algo;
            self.osc_a_l = Perc1Osc::new(self.sample_rate, algo);
            self.osc_a_r = Perc1Osc::new(self.sample_rate, algo);
            self.osc_b_l = Perc1Osc::new(self.sample_rate, algo);
            self.osc_b_r = Perc1Osc::new(self.sample_rate, algo);
        }
        self.osc_a_l.set_freq(base_freq);
        self.osc_a_r.set_freq(base_freq);
        self.osc_b_l.set_freq(base_freq * 1.5);
        self.osc_b_r.set_freq(base_freq * 1.5);

        self.rebuild_sweep();

        // Update amplitude envelope via setters — do NOT recreate to preserve tail state
        self.amp_env
            .set_decay(self.settings.decay.max(0.01).min(2.0));
        self.amp_env
            .set_attack_ms((self.settings.attack * 1000.0).max(MIN_AMP_ATTACK_MS));
        self.amp_env.set_release(self.settings.release.max(0.001));
        self.amp_env.set_decay_curve(self.settings.decay_curve);
        self.amp_env.set_release_curve(self.settings.release_curve);
        self.amp_env.set_hold(self.settings.hold);

        // Update filter envelope via setters — do NOT recreate to preserve tail state
        let filter_env_decay = self.settings.filter_env_decay.max(0.01).min(2.0);
        self.filter_env.set_decay(filter_env_decay);
        self.filter_env.set_curve(self.settings.decay_curve);

        // Update filter cutoff
        let filter_freq = self.settings.filter_freq.max(20.0).min(20000.0);
        self.filter.set_cutoff(filter_freq, self.sample_rate);

        // Update saturation
        self.saturation.saturation_type =
            saturation::SaturationType::from(self.settings.saturation_type);
        self.saturation.amount = self.settings.saturation_amount;
        self.saturation.mix = self.settings.saturation_mix;
        self.saturation.output_gain = self.settings.saturation_output_gain;
        self.saturation.pre_filter = self.settings.saturation_pre_filter > 0.5;
    }

    fn set_algo(&mut self, algo: u8) {
        self.settings.algo = algo;
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        match index {
            0 => self.settings.sweep = value,
            1 => self.settings.speed = value,
            2 => self.settings.bite = value,
            3 => self.settings.width = value,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_active_samples(voice: &mut Perc1Voice) -> usize {
        let mut count = 0;
        for _ in 0..(44100 * 5) {
            let (l, r) = voice.process_sample_stereo();
            if l != 0.0 || r != 0.0 {
                count += 1;
            }
        }
        count
    }

    /// Regression guard: a retrigger during a ringing tail must NOT reset the
    /// oscillator phases (that unconditional reset was the click parasite). The
    /// body should continue roughly continuously across the retrigger.
    #[test]
    fn perc1_no_click_on_retrigger_during_tail() {
        let sr = 44100.0;
        let mut settings = VoiceSettings::perc1();
        settings.decay = 0.3;
        settings.release = 0.4;
        settings.special[2] = 0.0; // bite (FM depth) off → isolate the carrier body
        settings.special[3] = 0.0; // width off → no slap-delay/stereo complication

        let mut voice = Perc1Voice::new(sr, settings.into());
        voice.trigger();
        let mut last = 0.0f32;
        for _ in 0..1500 {
            last = voice.process_sample();
        }
        assert!(last.abs() > 1e-4, "tail must still ring: {}", last);

        voice.trigger();
        let first = voice.process_sample();
        let edge = (first - last).abs();
        // The trigger-edge step is the click metric: a phase reset on the ringing
        // tail jumped the body hard here. The intra-sweep per-sample steps that
        // follow are the legitimate (fast) laser pitch sweep, not a discontinuity,
        // so we only assert on the edge.
        eprintln!(
            "perc1 retrigger: last={:.4} first={:.4} edge={:.4}",
            last, first, edge
        );
        assert!(
            edge < 0.05,
            "perc1 retrigger edge discontinuity (phase reset on tail?): {}",
            edge
        );
    }

    #[test]
    fn perc1_decay_release_are_audible() {
        let sr = 44100.0;
        let mut settings = VoiceSettings::perc1();

        // Short decay, no release
        settings.decay = 0.01;
        settings.release = 0.0;
        let mut voice = Perc1Voice::new(sr, settings.into());
        voice.trigger();
        let short = count_active_samples(&mut voice);

        // Long decay + long release
        settings.decay = 0.5;
        settings.release = 1.0;
        voice.set_settings(settings);
        voice.trigger();
        let long = count_active_samples(&mut voice);

        assert!(
            long > short * 2,
            "Long decay+release should produce significantly more samples than short (short={}, long={})",
            short, long
        );
    }

    #[test]
    fn perc1_hold_extends_active_duration() {
        let sr = 44100.0;
        let mut settings = VoiceSettings::perc1();
        settings.decay = 0.01;
        settings.release = 0.0;
        settings.hold = 0.0;
        settings.special[1] = 5.0;

        let mut voice = Perc1Voice::new(sr, settings.into());
        voice.trigger();
        let no_hold = count_active_samples(&mut voice);

        settings.hold = 0.1;
        voice.set_settings(settings);
        voice.trigger();
        let with_hold = count_active_samples(&mut voice);

        assert!(
            with_hold > no_hold + (sr * 0.05) as usize,
            "Hold should extend Perc1 active duration (no_hold={}, with_hold={})",
            no_hold,
            with_hold
        );
    }
}
