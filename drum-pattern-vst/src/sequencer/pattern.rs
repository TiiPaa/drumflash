//! Pattern data structures and shared realtime-safe state.

use nih_plug::params::persist::PersistentField;
use std::array;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

pub const INSTRUMENT_COUNT: usize = 7;
pub const STEP_COUNT: usize = 16;

/// A single step in a pattern containing trigger states for all instruments.
#[derive(Clone, Debug)]
pub struct Step {
    /// 7 instruments: Kick, Snare, HiHat, OpenHiHat, Tom1, Tom2, Tom3.
    pub instruments: [bool; INSTRUMENT_COUNT],
}

impl Step {
    pub fn new() -> Self {
        Self {
            instruments: [false; INSTRUMENT_COUNT],
        }
    }

    pub fn empty() -> Self {
        Self::new()
    }

    pub fn bitmask(&self) -> u8 {
        let mut bits = 0u8;
        for (instrument, active) in self.instruments.iter().copied().enumerate() {
            if active {
                bits |= 1 << instrument;
            }
        }
        bits
    }
}

/// A 16-step pattern for drum sequencing.
#[derive(Clone, Debug)]
pub struct Pattern {
    /// 16 steps, each with 7 instrument triggers.
    pub steps: [Step; STEP_COUNT],
    pub name: String,
}

impl Pattern {
    /// Create empty pattern.
    pub fn empty() -> Self {
        Self {
            steps: [
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
                Step::empty(),
            ],
            name: "Empty".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn default_pattern() -> Self {
        let mut pattern = Self::empty();
        pattern.name = "Default".to_string();

        pattern.steps[0].instruments[0] = true;
        pattern.steps[4].instruments[0] = true;
        pattern.steps[8].instruments[0] = true;
        pattern.steps[12].instruments[0] = true;

        pattern
    }

    pub fn rock_pattern() -> Self {
        let mut pattern = Self::empty();
        pattern.name = "Rock".to_string();

        pattern.steps[0].instruments[0] = true;
        pattern.steps[4].instruments[0] = true;
        pattern.steps[8].instruments[0] = true;
        pattern.steps[12].instruments[0] = true;

        pattern.steps[4].instruments[1] = true;
        pattern.steps[12].instruments[1] = true;

        for i in (0..STEP_COUNT).step_by(2) {
            pattern.steps[i].instruments[2] = true;
        }

        pattern
    }

    pub fn funk_pattern() -> Self {
        let mut pattern = Self::empty();
        pattern.name = "Funk".to_string();

        pattern.steps[0].instruments[0] = true;
        pattern.steps[3].instruments[0] = true;
        pattern.steps[6].instruments[0] = true;
        pattern.steps[10].instruments[0] = true;

        pattern.steps[4].instruments[1] = true;
        pattern.steps[7].instruments[1] = true;
        pattern.steps[12].instruments[1] = true;

        for i in 0..STEP_COUNT {
            if i % 3 != 0 {
                pattern.steps[i].instruments[2] = true;
            }
        }

        pattern.steps[2].instruments[3] = true;
        pattern.steps[6].instruments[3] = true;
        pattern.steps[10].instruments[3] = true;
        pattern.steps[14].instruments[3] = true;

        pattern
    }

    pub fn disco_pattern() -> Self {
        let mut pattern = Self::empty();
        pattern.name = "Disco".to_string();

        for beat in [0, 4, 8, 12] {
            pattern.steps[beat].instruments[0] = true;
        }

        pattern.steps[4].instruments[1] = true;
        pattern.steps[12].instruments[1] = true;

        for step in 0..STEP_COUNT {
            pattern.steps[step].instruments[2] = true;
        }

        for step in [3, 7, 11, 15] {
            pattern.steps[step].instruments[3] = true;
        }

        pattern
    }

    pub fn get_step(&self, index: usize) -> &Step {
        &self.steps[index % STEP_COUNT]
    }

    pub fn step_masks(&self) -> [u8; STEP_COUNT] {
        array::from_fn(|step| self.get_step(step).bitmask())
    }

    #[allow(dead_code)]
    pub fn toggle(&mut self, instrument: usize, step: usize) {
        if instrument < INSTRUMENT_COUNT && step < STEP_COUNT {
            self.steps[step].instruments[instrument] = !self.steps[step].instruments[instrument];
        }
    }

    #[allow(dead_code)]
    pub fn set(&mut self, instrument: usize, step: usize, value: bool) {
        if instrument < INSTRUMENT_COUNT && step < STEP_COUNT {
            self.steps[step].instruments[instrument] = value;
        }
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        for step in &mut self.steps {
            step.instruments = [false; INSTRUMENT_COUNT];
        }
    }

    pub fn random_pattern() -> Self {
        let mut pattern = Self::empty();
        pattern.name = "Random".to_string();

        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // Simple LCG for deterministic randomness without dependencies
        let mut rng = seed;
        let mut lcg = || {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            rng
        };

        for step in 0..STEP_COUNT {
            for instrument in 0..INSTRUMENT_COUNT {
                if (lcg() % 10) < 3 {
                    // 30% probability like the original PoC
                    pattern.steps[step].instruments[instrument] = true;
                }
            }
        }

        pattern
    }
}

/// Lock-free pattern storage shared between the audio thread and the UI.
pub struct SharedPattern {
    steps: [AtomicU8; STEP_COUNT],
}

#[derive(Clone)]
pub struct PersistentPattern {
    shared: Arc<SharedPattern>,
}

impl PersistentPattern {
    pub fn new(pattern: &Pattern) -> Self {
        Self {
            shared: SharedPattern::new(pattern),
        }
    }

    pub fn shared(&self) -> Arc<SharedPattern> {
        self.shared.clone()
    }
}

impl SharedPattern {
    pub fn new(pattern: &Pattern) -> Arc<Self> {
        let shared = Arc::new(Self {
            steps: array::from_fn(|step| AtomicU8::new(pattern.get_step(step).bitmask())),
        });
        shared
    }

    pub fn load_step_mask(&self, step: usize) -> u8 {
        self.steps[step % STEP_COUNT].load(Ordering::Relaxed)
    }

    pub fn set_step_mask(&self, step: usize, mask: u8) {
        self.steps[step % STEP_COUNT].store(mask & 0x7f, Ordering::Relaxed);
    }

    pub fn is_active(&self, step: usize, instrument: usize) -> bool {
        if instrument >= INSTRUMENT_COUNT {
            return false;
        }

        let mask = self.load_step_mask(step);
        (mask & (1 << instrument)) != 0
    }

    pub fn load_step_masks(&self, masks: &[u8; STEP_COUNT]) {
        for (step, mask) in masks.iter().copied().enumerate() {
            self.set_step_mask(step, mask);
        }
    }

    pub fn step_masks(&self) -> [u8; STEP_COUNT] {
        array::from_fn(|step| self.load_step_mask(step))
    }
}

impl<'a> PersistentField<'a, [u8; STEP_COUNT]> for PersistentPattern {
    fn set(&self, new_value: [u8; STEP_COUNT]) {
        self.shared.load_step_masks(&new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&[u8; STEP_COUNT]) -> R,
    {
        let masks = self.shared.step_masks();
        f(&masks)
    }
}
