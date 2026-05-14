#[derive(Debug, Clone, Copy)]
pub struct OnePoleSmoother {
    current: f32,
    coeff: f32,
}

impl OnePoleSmoother {
    pub fn new(sample_rate: f32, time_ms: f32, initial: f32) -> Self {
        let mut smoother = Self {
            current: initial,
            coeff: 0.0,
        };
        smoother.set_time_ms(sample_rate, time_ms);
        smoother
    }

    pub fn set_time_ms(&mut self, sample_rate: f32, time_ms: f32) {
        let time_ms = time_ms.max(0.01);
        let time_seconds = time_ms * 0.001;
        self.coeff = (-1.0 / (sample_rate * time_seconds)).exp();
    }

    pub fn reset(&mut self, value: f32) {
        self.current = value;
    }

    pub fn process(&mut self, target: f32) -> f32 {
        self.current = target + self.coeff * (self.current - target);
        self.current
    }
}
