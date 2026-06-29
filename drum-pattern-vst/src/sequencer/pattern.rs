//! Pattern data structures and shared realtime-safe state.

use nih_plug::params::persist::PersistentField;
use std::array;
use std::sync::{
    atomic::{AtomicU16, AtomicU64, Ordering},
    Arc,
};

/// Maximum fused groups per instrument in SharedPattern.
pub const MAX_FUSIONS: usize = 16;

/// Pack a FusedGroup into a u64 for atomic storage.
/// Bits 0-7: start_cell, 8-15: end_cell, 16-23: pulse_count, 24: active,
/// 25: morph_active, 26-31: morph_field, 32-63: morph_end_value (f32 bitcast).
pub(crate) fn pack_fusion(group: &FusedGroup) -> u64 {
    let end_bits = group.morph_end_value.to_bits() as u64;
    (group.start_cell as u64)
        | ((group.end_cell as u64) << 8)
        | ((group.step_count.clamp(1, 64) as u64) << 16)
        | (1u64 << 24)
        | ((group.morph_active() as u64) << 25)
        | ((group.morph_field as u64) << 26)
        | (end_bits << 32)
}

/// Unpack a u64 into an Option<FusedGroup>.
pub(crate) fn unpack_fusion(packed: u64) -> Option<FusedGroup> {
    if (packed >> 24) & 1 == 0 {
        return None;
    }
    let morph_field = ((packed >> 26) & 0x3F) as u8;
    let morph_end_value = f32::from_bits((packed >> 32) as u32);
    let group = FusedGroup {
        start_cell: (packed & 0xFF) as u8,
        end_cell: ((packed >> 8) & 0xFF) as u8,
        step_count: ((packed >> 16) & 0xFF) as u8,
        morph_field,
        morph_end_value,
    };
    group.is_valid().then_some(group)
}

pub const INSTRUMENT_COUNT: usize = 13;
pub const STEP_COUNT: usize = 64;

/// A single step in a pattern containing trigger states for all instruments.
#[derive(Clone, Debug)]
pub struct Step {
    ///  instruments: Kick, Snare, HiHat, OpenHiHat, Tom1, Tom2, Tom3, Clap, Ride, Cymbal, Snare606, BassDrum808, Zap.
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

    pub fn bitmask(&self) -> u16 {
        let mut bits = 0u16;
        for (instrument, active) in self.instruments.iter().copied().enumerate() {
            if active {
                bits |= 1 << instrument;
            }
        }
        bits
    }
}

/// A fused cell group for Step Fusion (tuplets / micro-rhythms).
///
/// A group is one UI button spanning consecutive cells on a single 16-step
/// page. When its start cell is active, the audio engine emits `step_count`
/// evenly-spaced pulses over the duration of the whole group. Sound plocks are
/// read from `start_cell`; sequencer stutter is ignored for fused cells.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FusedGroup {
    /// First cell index (0-63) inclusive.
    pub start_cell: u8,
    /// Last cell index (0-63) inclusive.
    pub end_cell: u8,
    /// Number of pulses generated across this group (1-64).
    pub step_count: u8,
    /// Plock field index to morph across pulses (255 = no morphing).
    pub morph_field: u8,
    /// Target value for the morphed field at the last pulse.
    pub morph_end_value: f32,
}

impl Default for FusedGroup {
    fn default() -> Self {
        Self {
            start_cell: 0,
            end_cell: 0,
            step_count: 0,
            morph_field: 255,
            morph_end_value: 0.0,
        }
    }
}

impl FusedGroup {
    pub fn cell_span(&self) -> usize {
        self.end_cell.saturating_sub(self.start_cell) as usize + 1
    }

    pub fn page(&self) -> usize {
        self.start_cell as usize / 16
    }

    pub fn is_page_local(&self) -> bool {
        self.start_cell / 16 == self.end_cell / 16
    }

    pub fn is_valid(&self) -> bool {
        let start = self.start_cell as usize;
        let end = self.end_cell as usize;
        start < end && end < STEP_COUNT && self.step_count >= 1 && self.is_page_local()
    }

    pub fn morph_active(&self) -> bool {
        self.morph_field != 255
    }

    pub fn morph_field_index(&self) -> Option<usize> {
        if self.morph_active() {
            Some(self.morph_field as usize)
        } else {
            None
        }
    }

    /// Returns true if the given cell index is inside this group.
    pub fn contains(&self, cell: usize) -> bool {
        cell >= self.start_cell as usize && cell <= self.end_cell as usize
    }

    pub fn is_start(&self, cell: usize) -> bool {
        cell == self.start_cell as usize
    }
}

/// A 64-step pattern for drum sequencing (4 pages of 16 steps).
#[derive(Clone, Debug)]
pub struct Pattern {
    ///  steps, each with  instrument triggers.
    pub steps: [Step; STEP_COUNT],
    pub name: String,
    /// Per-instrument fused cell groups (Step Fusion feature).
    pub fusions: [Vec<FusedGroup>; INSTRUMENT_COUNT],
}

impl Pattern {
    /// Create empty pattern.
    pub fn empty() -> Self {
        Self {
            steps: array::from_fn(|_| Step::empty()),
            name: "Empty".to_string(),
            fusions: array::from_fn(|_| Vec::new()),
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

        for bar in 0..(STEP_COUNT / 16) {
            let offset = bar * 16;
            pattern.steps[offset + 0].instruments[0] = true;
            pattern.steps[offset + 4].instruments[0] = true;
            pattern.steps[offset + 8].instruments[0] = true;
            pattern.steps[offset + 12].instruments[0] = true;

            pattern.steps[offset + 4].instruments[1] = true;
            pattern.steps[offset + 12].instruments[1] = true;
        }

        for i in (0..STEP_COUNT).step_by(2) {
            pattern.steps[i].instruments[2] = true;
        }

        pattern
    }

    pub fn funk_pattern() -> Self {
        let mut pattern = Self::empty();
        pattern.name = "Funk".to_string();

        for bar in 0..(STEP_COUNT / 16) {
            let offset = bar * 16;
            pattern.steps[offset + 0].instruments[0] = true;
            pattern.steps[offset + 3].instruments[0] = true;
            pattern.steps[offset + 6].instruments[0] = true;
            pattern.steps[offset + 10].instruments[0] = true;

            pattern.steps[offset + 4].instruments[1] = true;
            pattern.steps[offset + 7].instruments[1] = true;
            pattern.steps[offset + 12].instruments[1] = true;

            pattern.steps[offset + 2].instruments[3] = true;
            pattern.steps[offset + 6].instruments[3] = true;
            pattern.steps[offset + 10].instruments[3] = true;
            pattern.steps[offset + 14].instruments[3] = true;
        }

        for i in 0..STEP_COUNT {
            if i % 3 != 0 {
                pattern.steps[i].instruments[2] = true;
            }
        }

        pattern
    }

    pub fn disco_pattern() -> Self {
        let mut pattern = Self::empty();
        pattern.name = "Disco".to_string();

        for bar in 0..(STEP_COUNT / 16) {
            let offset = bar * 16;
            for beat in [0, 4, 8, 12] {
                pattern.steps[offset + beat].instruments[0] = true;
            }

            pattern.steps[offset + 4].instruments[1] = true;
            pattern.steps[offset + 12].instruments[1] = true;

            for step in [3, 7, 11, 15] {
                pattern.steps[offset + step].instruments[3] = true;
            }
        }

        for step in 0..STEP_COUNT {
            pattern.steps[step].instruments[2] = true;
        }

        pattern
    }

    pub fn get_step(&self, index: usize) -> &Step {
        &self.steps[index % STEP_COUNT]
    }

    pub fn step_masks(&self) -> [u16; STEP_COUNT] {
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
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0);

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
    steps: [AtomicU16; STEP_COUNT],
    /// Fused groups per instrument, packed as u64.
    /// Index: instrument * MAX_FUSIONS + group_index.
    /// The first element for each instrument stores the count (packed as count | (1<<24)).
    fusions: [AtomicU64; INSTRUMENT_COUNT * MAX_FUSIONS],
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternMasks(#[serde(with = "serde_arrays")] pub [u16; STEP_COUNT]);

/// Full pattern state persisted by the DAW: step masks + fused groups.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternState {
    #[serde(with = "serde_arrays")]
    pub masks: [u16; STEP_COUNT],
    #[serde(with = "serde_arrays")]
    pub fusions: [u64; INSTRUMENT_COUNT * MAX_FUSIONS],
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

impl<'a> PersistentField<'a, PatternState> for PersistentPattern {
    fn set(&self, new_value: PatternState) {
        self.shared.load_step_masks(&new_value.masks);
        for (i, &packed) in new_value.fusions.iter().enumerate() {
            self.shared.fusions[i].store(packed, Ordering::Relaxed);
        }
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&PatternState) -> R,
    {
        let masks = self.shared.step_masks();
        let fusions = std::array::from_fn(|i| self.shared.fusions[i].load(Ordering::Relaxed));
        f(&PatternState { masks, fusions })
    }
}

impl SharedPattern {
    pub fn new(pattern: &Pattern) -> Arc<Self> {
        let fusions = array::from_fn(|i| {
            let inst = i / MAX_FUSIONS;
            let group_idx = i % MAX_FUSIONS;
            if group_idx == 0 {
                // First slot stores count
                AtomicU64::new((pattern.fusions[inst].len() as u64) | (1u64 << 24))
            } else if group_idx <= pattern.fusions[inst].len() {
                AtomicU64::new(pack_fusion(&pattern.fusions[inst][group_idx - 1]))
            } else {
                AtomicU64::new(0)
            }
        });
        let shared = Arc::new(Self {
            steps: array::from_fn(|step| AtomicU16::new(pattern.get_step(step).bitmask())),
            fusions,
        });
        shared
    }

    /// Load fused groups for an instrument.
    pub fn load_fusions(&self, instrument: usize) -> Vec<FusedGroup> {
        if instrument >= INSTRUMENT_COUNT {
            return Vec::new();
        }
        let base = instrument * MAX_FUSIONS;
        let count_packed = self.fusions[base].load(Ordering::Relaxed);
        let count = (count_packed & 0xFF) as usize;
        let mut groups = Vec::with_capacity(count.min(MAX_FUSIONS - 1));
        for i in 0..count.min(MAX_FUSIONS - 1) {
            if let Some(group) = unpack_fusion(self.fusions[base + 1 + i].load(Ordering::Relaxed)) {
                groups.push(group);
            }
        }
        groups
    }

    /// Load fused groups into a caller-provided fixed buffer. This is used by
    /// the audio thread to avoid allocating while syncing UI state.
    pub fn load_fusions_into(
        &self,
        instrument: usize,
        out: &mut [FusedGroup; MAX_FUSIONS],
    ) -> usize {
        if instrument >= INSTRUMENT_COUNT {
            return 0;
        }
        let base = instrument * MAX_FUSIONS;
        let count_packed = self.fusions[base].load(Ordering::Relaxed);
        let count = (count_packed & 0xFF) as usize;
        let mut written = 0usize;
        for i in 0..count.min(MAX_FUSIONS - 1) {
            if let Some(group) = unpack_fusion(self.fusions[base + 1 + i].load(Ordering::Relaxed)) {
                if written < out.len() {
                    out[written] = group;
                    written += 1;
                }
            }
        }
        written
    }

    /// Store fused groups for an instrument.
    pub fn store_fusions(&self, instrument: usize, groups: &[FusedGroup]) {
        if instrument >= INSTRUMENT_COUNT {
            return;
        }
        let base = instrument * MAX_FUSIONS;
        let mut count = 0usize;
        for group in groups
            .iter()
            .copied()
            .filter(FusedGroup::is_valid)
            .take(MAX_FUSIONS - 1)
        {
            self.fusions[base + 1 + count].store(pack_fusion(&group), Ordering::Relaxed);
            count += 1;
        }
        // Clear remaining slots
        for i in count..(MAX_FUSIONS - 1) {
            self.fusions[base + 1 + i].store(0, Ordering::Relaxed);
        }
        // Update count
        self.fusions[base].store((count as u64) | (1u64 << 24), Ordering::Relaxed);
    }

    pub fn load_step_mask(&self, step: usize) -> u16 {
        self.steps[step % STEP_COUNT].load(Ordering::Relaxed)
    }

    pub fn set_step_mask(&self, step: usize, mask: u16) {
        // Mask bits to the number of instruments. Each bit position
        // corresponds to a voice index in `DrumVoice`. Wider masks lose any
        // active bits for voices beyond INSTRUMENT_COUNT, which is why
        // adding an 11th voice required widening this from 0x3ff (10 bits) to
        // accommodate the new bit 10 (Snare 606).
        let valid_bits = (1u16 << INSTRUMENT_COUNT).wrapping_sub(1);
        self.steps[step % STEP_COUNT].store(mask & valid_bits, Ordering::Relaxed);
    }

    pub fn is_active(&self, step: usize, instrument: usize) -> bool {
        if instrument >= INSTRUMENT_COUNT {
            return false;
        }

        let mask = self.load_step_mask(step);
        (mask & (1 << instrument)) != 0
    }

    pub fn load_step_masks(&self, masks: &[u16; STEP_COUNT]) {
        for (step, mask) in masks.iter().copied().enumerate() {
            self.set_step_mask(step, mask);
        }
    }

    pub fn step_masks(&self) -> [u16; STEP_COUNT] {
        array::from_fn(|step| self.load_step_mask(step))
    }
}
