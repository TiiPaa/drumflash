//! Pattern data structures and shared realtime-safe state.

use nih_plug::params::persist::PersistentField;
use std::array;
use std::sync::{
    atomic::{AtomicU16, AtomicU64, Ordering},
    Arc,
};

/// Maximum fused groups per instrument in SharedPattern.
pub const MAX_FUSIONS: usize = 16;

/// Number of AtomicU64 slots used per fused group in SharedPattern.
pub(crate) const FUSION_SLOT_COUNT: usize = 3;

/// A single morphing target inside a fused group.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MorphTarget {
    pub field: u8,
    pub end_value: f32,
}

impl Default for MorphTarget {
    fn default() -> Self {
        Self {
            field: 255,
            end_value: 0.0,
        }
    }
}

/// Pack a FusedGroup into 3 u64s for atomic storage.
///
/// New layout (bit 18 = valid, bit 24 = 0 to distinguish from old format):
/// Slot 0: bits 0-5 start_cell, 6-11 end_cell, 12-17 step_count-1, 18 valid,
///         19-21 morph_count, 22-29 target[0].field, 30-61 target[0].end_value.
/// Slot 1: bits 0-7 target[1].field, 8-39 target[1].end_value,
///         40-47 target[2].field, 48-63 target[2].end_value upper 16 bits.
/// Slot 2: bits 0-15 target[2].end_value lower 16 bits,
///         16-23 target[3].field, 24-55 target[3].end_value.
pub(crate) fn pack_fusion(group: &FusedGroup) -> [u64; FUSION_SLOT_COUNT] {
    let target =
        |i: usize| -> MorphTarget { group.morph_targets.get(i).copied().unwrap_or_default() };

    let t0 = target(0);
    let step_count_minus_1 = (group.step_count.clamp(1, 64) - 1) as u64;
    let slot0 = (group.start_cell as u64)
        | ((group.end_cell as u64) << 6)
        | (step_count_minus_1 << 12)
        | (1u64 << 18)
        | ((group.morph_count.clamp(0, 4) as u64) << 19)
        | ((t0.field as u64) << 22)
        | ((t0.end_value.to_bits() as u64) << 30);

    let t1 = target(1);
    let t2 = target(2);
    let t2_bits = t2.end_value.to_bits() as u64;
    let slot1 = (t1.field as u64)
        | ((t1.end_value.to_bits() as u64) << 8)
        | ((t2.field as u64) << 40)
        | ((t2_bits & 0xFFFF_0000) << 32); // upper 16 bits at 48-63

    let t3 = target(3);
    let slot2 = ((t2_bits & 0xFFFF) << 0)
        | ((t3.field as u64) << 16)
        | ((t3.end_value.to_bits() as u64) << 24);

    [slot0, slot1, slot2]
}

/// Unpack 3 u64s into an Option<FusedGroup>.
/// Only understands the current (fixed) layout; old pattern-v3 fusion data is
/// migrated at the state level in `lib.rs::filter_state`.
pub(crate) fn unpack_fusion(slots: [u64; FUSION_SLOT_COUNT]) -> Option<FusedGroup> {
    let slot0 = slots[0];
    if (slot0 >> 18) & 1 == 0 {
        return None;
    }

    let mut targets = [MorphTarget::default(); 4];
    let count = ((slot0 >> 19) & 0x7) as usize;
    if count > 0 {
        targets[0] = MorphTarget {
            field: ((slot0 >> 22) & 0xFF) as u8,
            end_value: f32::from_bits(((slot0 >> 30) & 0xFFFFFFFF) as u32),
        };
    }
    if count > 1 {
        let s1 = slots[1];
        targets[1] = MorphTarget {
            field: (s1 & 0xFF) as u8,
            end_value: f32::from_bits(((s1 >> 8) & 0xFFFFFFFF) as u32),
        };
    }
    if count > 2 {
        let s1 = slots[1];
        let s2 = slots[2];
        let field = ((s1 >> 40) & 0xFF) as u8;
        let value_upper = ((s1 >> 48) & 0xFFFF) as u32;
        let value_lower = (s2 & 0xFFFF) as u32;
        targets[2] = MorphTarget {
            field,
            end_value: f32::from_bits((value_upper << 16) | value_lower),
        };
    }
    if count > 3 {
        let s2 = slots[2];
        targets[3] = MorphTarget {
            field: ((s2 >> 16) & 0xFF) as u8,
            end_value: f32::from_bits(((s2 >> 24) & 0xFFFFFFFF) as u32),
        };
    }

    let group = FusedGroup {
        start_cell: (slot0 & 0x3F) as u8,
        end_cell: ((slot0 >> 6) & 0x3F) as u8,
        step_count: (((slot0 >> 12) & 0x3F) as u8).saturating_add(1),
        morph_count: count.clamp(0, 4) as u8,
        morph_targets: targets,
    };
    group.is_valid().then_some(group)
}

/// Unpack the old broken pattern-v3 fusion layout (valid bit 24) and return
/// a geometry-only group. Used by `filter_state` to migrate saved DAW state.
pub(crate) fn unpack_fusion_v3_old(slots: [u64; FUSION_SLOT_COUNT]) -> Option<FusedGroup> {
    let meta = slots[0];
    if (meta >> 24) & 1 == 0 {
        return None;
    }
    let mut group = FusedGroup::default();
    group.start_cell = (meta & 0xFF) as u8;
    group.end_cell = ((meta >> 8) & 0xFF) as u8;
    group.step_count = ((meta >> 16) & 0xFF) as u8;
    group.morph_count = 0;
    group.morph_targets = [MorphTarget::default(); 4];
    group.is_valid().then_some(group)
}

/// Unpack a legacy single-u64 fusion (pattern-v3 before multi-target morphing).
/// Used for pattern-bank backward compatibility.
pub(crate) fn unpack_fusion_legacy(packed: u64) -> Option<FusedGroup> {
    if (packed >> 24) & 1 == 0 {
        return None;
    }
    let morph_field = ((packed >> 26) & 0x3F) as u8;
    let morph_end_value = f32::from_bits((packed >> 32) as u32);
    let mut targets = [MorphTarget::default(); 4];
    let mut count = 0u8;
    if morph_field != 255 {
        targets[0] = MorphTarget {
            field: morph_field,
            end_value: morph_end_value,
        };
        count = 1;
    }
    let group = FusedGroup {
        start_cell: (packed & 0xFF) as u8,
        end_cell: ((packed >> 8) & 0xFF) as u8,
        step_count: ((packed >> 16) & 0xFF) as u8,
        morph_count: count,
        morph_targets: targets,
    };
    group.is_valid().then_some(group)
}

pub const INSTRUMENT_COUNT: usize = 14;
pub const STEP_COUNT: usize = 64;

/// Legacy 13-instrument pattern state used for migrating `pattern-v3` to `pattern-v5`.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternStateV3 {
    #[serde(with = "serde_arrays")]
    pub masks: [u16; STEP_COUNT],
    #[serde(with = "serde_arrays")]
    pub fusions: [u64; 13 * MAX_FUSIONS * FUSION_SLOT_COUNT],
}

impl PatternStateV3 {
    /// Expand a 13-instrument pattern state into the current 14-instrument format.
    /// The 14th instrument row is initialized to empty.
    pub fn expand(self) -> PatternState {
        let mut masks = [0u16; STEP_COUNT];
        masks.copy_from_slice(&self.masks);
        let mut fusions = [0u64; INSTRUMENT_COUNT * MAX_FUSIONS * FUSION_SLOT_COUNT];
        for inst in 0..13 {
            let old_base = inst * MAX_FUSIONS * FUSION_SLOT_COUNT;
            let new_base = inst * MAX_FUSIONS * FUSION_SLOT_COUNT;
            for i in 0..(MAX_FUSIONS * FUSION_SLOT_COUNT) {
                fusions[new_base + i] = self.fusions[old_base + i];
            }
        }
        PatternState { masks, fusions }
    }
}

/// Legacy 13-instrument pattern state used for migrating `pattern-v4` to `pattern-v5`.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternStateV4 {
    #[serde(with = "serde_arrays")]
    pub masks: [u16; STEP_COUNT],
    #[serde(with = "serde_arrays")]
    pub fusions: [u64; 13 * MAX_FUSIONS * FUSION_SLOT_COUNT],
}

impl PatternStateV4 {
    /// Expand a 13-instrument pattern state into the current 14-instrument format.
    /// The 14th instrument row is initialized to empty.
    pub fn expand(self) -> PatternState {
        let mut masks = [0u16; STEP_COUNT];
        masks.copy_from_slice(&self.masks);
        let mut fusions = [0u64; INSTRUMENT_COUNT * MAX_FUSIONS * FUSION_SLOT_COUNT];
        for inst in 0..13 {
            let old_base = inst * MAX_FUSIONS * FUSION_SLOT_COUNT;
            let new_base = inst * MAX_FUSIONS * FUSION_SLOT_COUNT;
            for i in 0..(MAX_FUSIONS * FUSION_SLOT_COUNT) {
                fusions[new_base + i] = self.fusions[old_base + i];
            }
        }
        PatternState { masks, fusions }
    }
}

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
    /// Number of active morphing targets (0-4).
    pub morph_count: u8,
    /// Morphing targets. Only the first `morph_count` entries are valid.
    pub morph_targets: [MorphTarget; 4],
}

impl Default for FusedGroup {
    fn default() -> Self {
        Self {
            start_cell: 0,
            end_cell: 0,
            step_count: 0,
            morph_count: 0,
            morph_targets: [MorphTarget::default(); 4],
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
        self.morph_count > 0
    }

    /// Returns true if the given field index is already a morph target.
    pub fn has_morph_target(&self, field: usize) -> bool {
        self.morph_targets[..self.morph_count as usize]
            .iter()
            .any(|t| t.field == field as u8)
    }

    /// Add or update a morph target, preserving order and capping at 4.
    pub fn set_morph_target(&mut self, field: usize, end_value: f32) {
        let field = field as u8;
        if let Some(existing) = self.morph_targets[..self.morph_count as usize]
            .iter_mut()
            .find(|t| t.field == field)
        {
            existing.end_value = end_value;
            return;
        }
        if self.morph_count < 4 {
            self.morph_targets[self.morph_count as usize] = MorphTarget { field, end_value };
            self.morph_count += 1;
        }
    }

    /// Remove a morph target if it exists.
    pub fn remove_morph_target(&mut self, field: usize) {
        let field = field as u8;
        let count = self.morph_count as usize;
        if let Some(pos) = self.morph_targets[..count]
            .iter()
            .position(|t| t.field == field)
        {
            for i in pos..count.saturating_sub(1) {
                self.morph_targets[i] = self.morph_targets[i + 1];
            }
            self.morph_targets[count.saturating_sub(1)] = MorphTarget::default();
            self.morph_count = self.morph_count.saturating_sub(1);
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
    /// Fused groups per instrument, packed as `FUSION_SLOT_COUNT` u64s per group.
    /// Index: instrument * MAX_FUSIONS * FUSION_SLOT_COUNT + group_index * FUSION_SLOT_COUNT + slot.
    /// The first slot for each instrument stores the count (packed as count | (1<<24)).
    fusions: [AtomicU64; INSTRUMENT_COUNT * MAX_FUSIONS * FUSION_SLOT_COUNT],
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternMasks(#[serde(with = "serde_arrays")] pub [u16; STEP_COUNT]);

/// Full pattern state persisted by the DAW: step masks + fused groups.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternState {
    #[serde(with = "serde_arrays")]
    pub masks: [u16; STEP_COUNT],
    #[serde(with = "serde_arrays")]
    pub fusions: [u64; INSTRUMENT_COUNT * MAX_FUSIONS * FUSION_SLOT_COUNT],
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
            let inst = i / (MAX_FUSIONS * FUSION_SLOT_COUNT);
            let rem = i % (MAX_FUSIONS * FUSION_SLOT_COUNT);
            let group_idx = rem / FUSION_SLOT_COUNT;
            let slot = rem % FUSION_SLOT_COUNT;
            if group_idx == 0 && slot == 0 {
                // First slot stores count
                AtomicU64::new((pattern.fusions[inst].len() as u64) | (1u64 << 24))
            } else if group_idx > 0 && group_idx <= pattern.fusions[inst].len() {
                let packed = pack_fusion(&pattern.fusions[inst][group_idx - 1]);
                AtomicU64::new(packed[slot])
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
        let base = instrument * MAX_FUSIONS * FUSION_SLOT_COUNT;
        let count_packed = self.fusions[base].load(Ordering::Relaxed);
        let count = (count_packed & 0xFF) as usize;
        let mut groups = Vec::with_capacity(count.min(MAX_FUSIONS - 1));
        for i in 0..count.min(MAX_FUSIONS - 1) {
            let slot_base = base + (1 + i) * FUSION_SLOT_COUNT;
            let slots = [
                self.fusions[slot_base].load(Ordering::Relaxed),
                self.fusions[slot_base + 1].load(Ordering::Relaxed),
                self.fusions[slot_base + 2].load(Ordering::Relaxed),
            ];
            if let Some(group) = unpack_fusion(slots) {
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
        let base = instrument * MAX_FUSIONS * FUSION_SLOT_COUNT;
        let count_packed = self.fusions[base].load(Ordering::Relaxed);
        let count = (count_packed & 0xFF) as usize;
        let mut written = 0usize;
        for i in 0..count.min(MAX_FUSIONS - 1) {
            let slot_base = base + (1 + i) * FUSION_SLOT_COUNT;
            let slots = [
                self.fusions[slot_base].load(Ordering::Relaxed),
                self.fusions[slot_base + 1].load(Ordering::Relaxed),
                self.fusions[slot_base + 2].load(Ordering::Relaxed),
            ];
            if let Some(group) = unpack_fusion(slots) {
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
        let base = instrument * MAX_FUSIONS * FUSION_SLOT_COUNT;
        let mut count = 0usize;
        for group in groups
            .iter()
            .copied()
            .filter(FusedGroup::is_valid)
            .take(MAX_FUSIONS - 1)
        {
            let packed = pack_fusion(&group);
            let slot_base = base + (1 + count) * FUSION_SLOT_COUNT;
            for (slot, &value) in packed.iter().enumerate() {
                self.fusions[slot_base + slot].store(value, Ordering::Relaxed);
            }
            count += 1;
        }
        // Clear remaining slots
        for i in count..(MAX_FUSIONS - 1) {
            let slot_base = base + (1 + i) * FUSION_SLOT_COUNT;
            for slot in 0..FUSION_SLOT_COUNT {
                self.fusions[slot_base + slot].store(0, Ordering::Relaxed);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_group(morph_targets: &[(usize, f32)], step_count: u8) -> FusedGroup {
        let mut group = FusedGroup {
            start_cell: 0,
            end_cell: step_count,
            step_count,
            morph_count: morph_targets.len().min(4) as u8,
            morph_targets: [MorphTarget::default(); 4],
        };
        for (i, &(field, value)) in morph_targets.iter().enumerate().take(4) {
            group.morph_targets[i] = MorphTarget {
                field: field as u8,
                end_value: value,
            };
        }
        group
    }

    #[test]
    fn pack_unpack_roundtrip_single_target() {
        let group = make_group(&[(0, 300.0)], 4);
        let packed = pack_fusion(&group);
        let unpacked = unpack_fusion(packed).unwrap();
        assert_eq!(group, unpacked);
    }

    #[test]
    fn pack_unpack_roundtrip_four_targets() {
        let group = make_group(&[(0, 300.0), (1, 2.5), (3, 8000.0), (18, 0.05)], 8);
        let packed = pack_fusion(&group);
        let unpacked = unpack_fusion(packed).unwrap();
        assert_eq!(group, unpacked);
    }

    #[test]
    fn pack_unpack_preserves_frequency_tom1() {
        let group = make_group(&[(0, 300.0)], 4);
        let packed = pack_fusion(&group);
        let unpacked = unpack_fusion(packed).unwrap();
        assert_eq!(unpacked.morph_targets[0].end_value, 300.0);
        assert!((unpacked.morph_targets[0].end_value - 300.0).abs() < f32::EPSILON);
    }

    #[test]
    fn old_broken_format_returns_none_at_fusion_level() {
        // Old layout is no longer decoded here; migration happens in filter_state.
        let old_meta: u64 = (0u64)
            | (2u64 << 0)   // start_cell
            | (6u64 << 8)   // end_cell
            | (4u64 << 16)  // step_count
            | (1u64 << 24); // valid
        let slots = [old_meta, 0, 0];
        assert!(unpack_fusion(slots).is_none());
    }

    #[test]
    fn shared_pattern_store_load_fusion_no_morph_roundtrip() {
        let pattern = SharedPattern::new(&Pattern::empty());
        let group = FusedGroup {
            start_cell: 2,
            end_cell: 6,
            step_count: 4,
            morph_count: 0,
            morph_targets: [MorphTarget::default(); 4],
        };
        pattern.store_fusions(4, &[group]);
        let loaded = pattern.load_fusions(4);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], group);
    }

    #[test]
    fn shared_pattern_store_load_fusion_roundtrip() {
        let pattern = SharedPattern::new(&Pattern::empty());
        let group = FusedGroup {
            start_cell: 2,
            end_cell: 6,
            step_count: 4,
            morph_count: 1,
            morph_targets: [
                MorphTarget {
                    field: 0,
                    end_value: 300.0,
                },
                MorphTarget::default(),
                MorphTarget::default(),
                MorphTarget::default(),
            ],
        };
        pattern.store_fusions(4, &[group]);
        let loaded = pattern.load_fusions(4);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], group);
    }

    #[test]
    fn invalid_fusion_returns_none() {
        let slots = [0u64; 3];
        assert!(unpack_fusion(slots).is_none());
    }
}
