use crate::dsp::{
    soft_clip, DcBlocker, ExpDecayEnvelope, NoiseBurstTransient, OnePoleSmoother,
};

#[derive(Debug, Clone, Copy)]
pub struct Kick808Params {
    pub base_freq_hz: f32,
    pub start_freq_hz: f32,
    pub pitch_decay_ms: f32,
    pub amp_decay_ms: f32,

    pub transient_level: f32,
    pub transient_decay_ms: f32,
    pub transient_brightness: f32,

    pub retrigger_peak: f32,
    pub tail_duck_amount: f32,
    pub tail_duck_ms: f32,

    pub freq_smoothing_ms: f32,
    pub drive: f32,
}

impl Default for Kick808Params {
    fn default() -> Self {
        Self {
            base_freq_hz: 52.0,
            start_freq_hz: 150.0,
            pitch_decay_ms: 28.0,
            amp_decay_ms: 650.0,

            transient_level: 0.18,
            transient_decay_ms: 1.2,
            transient_brightness: 0.30,

            retrigger_peak: 1.0,
            tail_duck_amount: 0.12,
            tail_duck_ms: 0.7,

            freq_smoothing_ms: 0.10,
            drive: 0.08,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Kick808Voice {
    sample_rate: f32,
    phase: f32,
    amp_env: ExpDecayEnvelope,
    pitch_env: ExpDecayEnvelope,
    tail_duck: ExpDecayEnvelope,
    freq_smoother: OnePoleSmoother,
    transient: NoiseBurstTransient,
    dc_block: DcBlocker,
}

impl Kick808Voice {
    pub fn new(sample_rate: f32) -> Self {
        let params = Kick808Params::default();
        Self {
            sample_rate,
            phase: 0.0,
            amp_env: ExpDecayEnvelope::new(sample_rate, params.amp_decay_ms),
            pitch_env: ExpDecayEnvelope::new(sample_rate, params.pitch_decay_ms),
            tail_duck: ExpDecayEnvelope::new(sample_rate, params.tail_duck_ms),
            freq_smoother: OnePoleSmoother::new(
                sample_rate,
                params.freq_smoothing_ms,
                params.base_freq_hz,
            ),
            transient: NoiseBurstTransient::new(sample_rate, params.transient_decay_ms),
            dc_block: DcBlocker::default(),
        }
    }

    pub fn trigger(&mut self, velocity: f32, params: &Kick808Params) {
        let velocity = velocity.clamp(0.0, 1.5);

        self.amp_env.set_decay_ms(self.sample_rate, params.amp_decay_ms);
        self.pitch_env
            .set_decay_ms(self.sample_rate, params.pitch_decay_ms);
        self.tail_duck
            .set_decay_ms(self.sample_rate, params.tail_duck_ms);
        self.freq_smoother
            .set_time_ms(self.sample_rate, params.freq_smoothing_ms);
        self.transient
            .set_decay_ms(self.sample_rate, params.transient_decay_ms);

        self.transient.trigger();

        let pitch_peak = (params.start_freq_hz - params.base_freq_hz).max(0.0);
        self.pitch_env.trigger_from_current(pitch_peak);

        let retrigger_peak = (velocity * params.retrigger_peak).max(self.amp_env.current());
        self.amp_env.trigger_from_current(retrigger_peak);

        self.tail_duck
            .trigger_from_current(params.tail_duck_amount.clamp(0.0, 0.95));
    }

    pub fn process(&mut self, params: &Kick808Params) -> f32 {
        let target_freq = (params.base_freq_hz + self.pitch_env.next()).max(10.0);
        let freq_hz = self.freq_smoother.process(target_freq);

        self.phase += freq_hz / self.sample_rate;
        self.phase -= self.phase.floor();

        let amp = self.amp_env.next();
        let duck_gain = 1.0 - self.tail_duck.next().clamp(0.0, 0.95);
        let body = (core::f32::consts::TAU * self.phase).sin() * amp * duck_gain;

        let transient = self
            .transient
            .next(params.transient_level, params.transient_brightness);

        let y = soft_clip(body + transient, params.drive);
        self.dc_block.process(y)
    }
}

#[cfg(test)]
mod tests {
    use super::{Kick808Params, Kick808Voice};

    #[test]
    fn retrigger_sequence_stays_finite() {
        let sr = 48_000.0;
        let mut voice = Kick808Voice::new(sr);
        let params = Kick808Params::default();

        let triggers = [0usize, 2_400, 4_800, 4_960, 9_600, 9_840];
        let mut trigger_index = 0usize;

        let mut peak = 0.0f32;
        for n in 0..12_000 {
            if trigger_index < triggers.len() && triggers[trigger_index] == n {
                voice.trigger(1.0, &params);
                trigger_index += 1;
            }
            let sample = voice.process(&params);
            assert!(sample.is_finite());
            peak = peak.max(sample.abs());
        }

        assert!(peak > 0.01);
        assert!(peak < 2.0);
    }
}
