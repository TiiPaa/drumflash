#[derive(Debug, Clone, Copy)]
pub struct DcBlocker {
    r: f32,
    x1: f32,
    y1: f32,
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self {
            r: 0.995,
            x1: 0.0,
            y1: 0.0,
        }
    }
}

impl DcBlocker {
    pub fn new(r: f32) -> Self {
        Self { r, ..Self::default() }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}
