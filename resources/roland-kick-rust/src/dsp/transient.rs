use super::envelope::ExpDecayEnvelope;

#[derive(Debug, Clone, Copy)]
pub struct NoiseBurstTransient {
    env: ExpDecayEnvelope,
    seed: u32,
    lowpass_state: f32,
}

impl NoiseBurstTransient {
    pub fn new(sample_rate: f32, decay_ms: f32) -> Self {
        Self {
            env: ExpDecayEnvelope::new(sample_rate, decay_ms),
            seed: 0x1234_5678,
            lowpass_state: 0.0,
        }
    }

    pub fn set_decay_ms(&mut self, sample_rate: f32, decay_ms: f32) {
        self.env.set_decay_ms(sample_rate, decay_ms);
    }

    pub fn trigger(&mut self) {
        self.env.set_immediate(1.0);
    }

    pub fn next(&mut self, level: f32, brightness: f32) -> f32 {
        if !self.env.is_active() {
            return 0.0;
        }

        let env = self.env.next();
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);

        let white = (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        let brightness = brightness.clamp(0.0, 1.0);
        let lp_coeff = 0.03 + brightness * 0.35;
        self.lowpass_state += lp_coeff * (white - self.lowpass_state);

        let high = white - self.lowpass_state;
        let coloured = high * brightness + self.lowpass_state * (1.0 - brightness);

        coloured * env * level
    }
}
