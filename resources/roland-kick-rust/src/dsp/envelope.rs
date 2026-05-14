#[derive(Debug, Clone, Copy)]
pub struct ExpDecayEnvelope {
    value: f32,
    coeff: f32,
}

impl Default for ExpDecayEnvelope {
    fn default() -> Self {
        Self {
            value: 0.0,
            coeff: 0.999,
        }
    }
}

impl ExpDecayEnvelope {
    pub fn new(sample_rate: f32, decay_ms: f32) -> Self {
        let mut env = Self::default();
        env.set_decay_ms(sample_rate, decay_ms);
        env
    }

    pub fn set_decay_ms(&mut self, sample_rate: f32, decay_ms: f32) {
        let decay_ms = decay_ms.max(0.01);
        let decay_seconds = decay_ms * 0.001;
        self.coeff = (-1.0 / (sample_rate * decay_seconds)).exp();
    }

    pub fn trigger_from_current(&mut self, peak: f32) {
        self.value = self.value.max(peak.max(0.0));
    }

    pub fn set_immediate(&mut self, value: f32) {
        self.value = value.max(0.0);
    }

    pub fn current(&self) -> f32 {
        self.value
    }

    pub fn is_active(&self) -> bool {
        self.value > 1.0e-6
    }

    pub fn next(&mut self) -> f32 {
        let out = self.value;
        self.value *= self.coeff;
        if self.value < 1.0e-6 {
            self.value = 0.0;
        }
        out
    }
}
