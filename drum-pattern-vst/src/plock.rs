//! Parameter locks (plocks) — per-step instrument sound overrides.
//!
//! Each instrument × step can store a complete override of the 12 sound settings
//! fields. When a step triggers, the audio thread applies the plock settings to
//! the voice before calling `trigger()`. When no plock is present, the voice
//! falls back to the global instrument settings.
//!
//! Two creation modes are supported:
//! - **Snapshot**: all fields are copied from global settings and locked.
//! - **Link**: only explicitly modified fields override globals; unmodified
//!   fields follow live global values.

use nih_plug::params::persist::PersistentField;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use crate::sequencer::pattern::INSTRUMENT_COUNT;
use crate::synthesis::VoiceSettings;

pub const STEP_COUNT: usize = 64;
// The field layout is defined by `param_id`, the module that owns the mapping
// between a parameter and its slot ([184]); re-exported here so every existing
// `crate::plock::FIELD_COUNT`-style path keeps working.
pub use crate::param_id::{
    ALGO_FIELD, ATTACK_FIELD, FIELD_COUNT, LEGACY_CLAP_ECHO_FIELD, SPECIAL_FIELD_COUNT,
    SPECIAL_FIELD_START,
};
const LEGACY_FIELD_COUNT: usize = 18;

/// Active-bit mask: one u64 per instrument (bit = step has a plock).
/// Stored as atomic so the UI can toggle bits without locking.
pub struct PlockMasks {
    pub masks: [AtomicU64; INSTRUMENT_COUNT],
}

impl PlockMasks {
    pub fn new() -> Self {
        Self {
            masks: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    pub fn is_active(&self, instrument: usize, step: usize) -> bool {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return false;
        }
        let mask = self.masks[instrument].load(Ordering::Acquire);
        (mask & (1u64 << step)) != 0
    }

    pub fn set_active(&self, instrument: usize, step: usize, active: bool) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        if active {
            self.masks[instrument].fetch_or(1u64 << step, Ordering::Release);
        } else {
            self.masks[instrument].fetch_and(!(1u64 << step), Ordering::Release);
        }
    }
}

/// Lock-free storage for plock values.
/// `values[instrument][step][field]` is a f32 bitcast.
/// Only meaningful when the corresponding bit in `PlockMasks` is set.
pub struct PlockValues {
    pub values: Vec<Vec<Vec<AtomicU64>>>,
}

impl PlockValues {
    pub fn new() -> Self {
        Self {
            values: (0..INSTRUMENT_COUNT)
                .map(|_| {
                    (0..STEP_COUNT)
                        .map(|_| (0..FIELD_COUNT).map(|_| AtomicU64::new(0)).collect())
                        .collect()
                })
                .collect(),
        }
    }

    pub fn get(&self, instrument: usize, step: usize, field: usize) -> f32 {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT || field >= FIELD_COUNT {
            return 0.0;
        }
        f32::from_bits(self.values[instrument][step][field].load(Ordering::Relaxed) as u32)
    }

    pub fn set(&self, instrument: usize, step: usize, field: usize, value: f32) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT || field >= FIELD_COUNT {
            return;
        }
        self.values[instrument][step][field].store(value.to_bits() as u64, Ordering::Relaxed);
    }
}

/// Per-field modification mask: one u32 per instrument × step.
/// Bit `1 << field` is set when that field has been explicitly overridden.
pub struct PlockFieldMasks {
    masks: Vec<Vec<AtomicU64>>,
}

impl PlockFieldMasks {
    pub fn new() -> Self {
        Self {
            masks: (0..INSTRUMENT_COUNT)
                .map(|_| (0..STEP_COUNT).map(|_| AtomicU64::new(0)).collect())
                .collect(),
        }
    }

    pub fn get(&self, instrument: usize, step: usize) -> u64 {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return 0;
        }
        self.masks[instrument][step].load(Ordering::Relaxed)
    }

    pub fn set(&self, instrument: usize, step: usize, field: usize) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT || field >= 64 {
            return;
        }
        let mask = self.masks[instrument][step].load(Ordering::Relaxed);
        self.masks[instrument][step].store(mask | (1u64 << field), Ordering::Relaxed);
    }

    pub fn clear(&self, instrument: usize, step: usize, field: usize) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT || field >= 64 {
            return;
        }
        let mask = self.masks[instrument][step].load(Ordering::Relaxed);
        self.masks[instrument][step].store(mask & !(1u64 << field), Ordering::Relaxed);
    }

    pub fn is_set(&self, instrument: usize, step: usize, field: usize) -> bool {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT || field >= 64 {
            return false;
        }
        let mask = self.masks[instrument][step].load(Ordering::Relaxed);
        (mask & (1u64 << field)) != 0
    }

    pub fn set_all(&self, instrument: usize, step: usize) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.masks[instrument][step].store((1u64 << FIELD_COUNT) - 1, Ordering::Relaxed);
    }

    pub fn set_legacy_snapshot(&self, instrument: usize, step: usize) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.masks[instrument][step].store((1u64 << LEGACY_FIELD_COUNT) - 1, Ordering::Relaxed);
    }

    pub fn clear_all(&self, instrument: usize, step: usize) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.masks[instrument][step].store(0, Ordering::Relaxed);
    }

    pub fn get_raw(&self, instrument: usize, step: usize) -> u64 {
        self.get(instrument, step)
    }

    /// Store a raw mask, repaired if it was written by an older build ([187]).
    ///
    /// This is the single choke point for every mask that enters the state from
    /// outside: DAW state, pattern-bank slots, presets, the page clipboard, a
    /// lane reorder, a step move, a p-lock paste.
    pub fn set_raw(&self, instrument: usize, step: usize, mask: u64) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.masks[instrument][step].store(
            crate::param_id::sanitize_field_mask(mask),
            Ordering::Relaxed,
        );
    }
}

/// Combined plock state exposed to UI and audio thread.
pub struct PlockState {
    pub masks: PlockMasks,
    pub values: PlockValues,
    pub field_masks: PlockFieldMasks,
}

impl PlockState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            masks: PlockMasks::new(),
            values: PlockValues::new(),
            field_masks: PlockFieldMasks::new(),
        })
    }

    /// Retrieve merged VoiceSettings for a step/instrument.
    /// Fields whose bit is set in `field_masks` come from the plock storage;
    /// all other fields fall back to `global`.
    /// Returns `None` when the step has no plock at all.
    pub fn get_settings(
        &self,
        instrument: usize,
        step: usize,
        global: &VoiceSettings,
    ) -> Option<VoiceSettings> {
        if !self.masks.is_active(instrument, step) {
            return None;
        }
        let mask = self.field_masks.get(instrument, step);
        if mask == 0 {
            // Link mode: no fields overridden yet
            return Some(*global);
        }

        let mut result = *global;
        let v = &self.values;

        if mask & (1 << 0) != 0 {
            result.frequency = v.get(instrument, step, 0);
        }
        if mask & (1 << 1) != 0 {
            result.decay = v.get(instrument, step, 1);
        }
        if mask & (1 << 2) != 0 {
            result.volume = v.get(instrument, step, 2);
        }
        if mask & (1 << 3) != 0 {
            result.filter_freq = v.get(instrument, step, 3);
        }
        if mask & (1 << 4) != 0 {
            result.release = v.get(instrument, step, 4);
        }
        if mask & (1 << 5) != 0 {
            result.decay_curve = v.get(instrument, step, 5);
        }
        if mask & (1 << 6) != 0 {
            result.release_curve = v.get(instrument, step, 6);
        }
        if mask & (1 << 7) != 0 {
            result.hold = v.get(instrument, step, 7);
        }
        if mask & (1 << 8) != 0 {
            result.filter_env_amount = v.get(instrument, step, 8);
        }
        if mask & (1 << 9) != 0 {
            result.filter_env_decay = v.get(instrument, step, 9);
        }
        if mask & (1 << 10) != 0 {
            result.analog = v.get(instrument, step, 10);
        }
        if mask & (1 << 11) != 0 {
            result.stereo = v.get(instrument, step, 11);
        }
        if mask & (1 << 13) != 0 {
            result.algo = v.get(instrument, step, ALGO_FIELD) as u8;
        }
        if mask & (1 << ATTACK_FIELD) != 0 {
            result.attack = v.get(instrument, step, ATTACK_FIELD);
        }

        for i in 0..SPECIAL_FIELD_COUNT {
            // [187] One mapping decides where a special lives, including index 4
            // (re-homed off Attack's field) and index 31 (which lends its slot).
            let Some(field) = crate::param_id::ParamId::Special(i).plock_field() else {
                continue;
            };
            if mask & (1u64 << field) != 0 {
                result.special[i] = v.get(instrument, step, field);
            }
        }

        // Legacy clap echo fallback (old presets stored echo in field 12)
        if instrument == 7 && result.special[0] == 0.0 {
            let legacy_clap_echo = v.get(instrument, step, LEGACY_CLAP_ECHO_FIELD);
            if legacy_clap_echo != 0.0 {
                result.special[0] = legacy_clap_echo;
            }
        }

        Some(result)
    }

    /// Store a complete VoiceSettings override for a step/instrument (snapshot mode).
    /// All fields are marked as overridden.
    pub fn set_settings(&self, instrument: usize, step: usize, settings: &VoiceSettings) {
        let v = &self.values;
        v.set(instrument, step, 0, settings.frequency);
        v.set(instrument, step, 1, settings.decay);
        v.set(instrument, step, 2, settings.volume);
        v.set(instrument, step, 3, settings.filter_freq);
        v.set(instrument, step, 4, settings.release);
        v.set(instrument, step, 5, settings.decay_curve);
        v.set(instrument, step, 6, settings.release_curve);
        v.set(instrument, step, 7, settings.hold);
        v.set(instrument, step, 8, settings.filter_env_amount);
        v.set(instrument, step, 9, settings.filter_env_decay);
        v.set(instrument, step, 10, settings.analog);
        v.set(instrument, step, 11, settings.stereo);
        v.set(instrument, step, ATTACK_FIELD, settings.attack);
        if instrument == 7 {
            v.set(instrument, step, LEGACY_CLAP_ECHO_FIELD, 0.0);
        }
        v.set(instrument, step, ALGO_FIELD, settings.algo as f32);
        for index in 0..SPECIAL_FIELD_COUNT.min(settings.special.len()) {
            // [187] Same single mapping as `get_settings`.
            let Some(field) = crate::param_id::ParamId::Special(index).plock_field() else {
                continue;
            };
            v.set(instrument, step, field, settings.special[index]);
        }
        // [187] Marks every ADDRESSABLE field, i.e. all but the dead legacy
        // clap-echo slot. Distinguishable from the old `set_all()` (all 46 bits),
        // which is what lets `sanitize_field_mask` spot a mask written before the
        // special-4 re-homing and refuse to trust its field 45.
        self.field_masks
            .set_raw(instrument, step, crate::param_id::ADDRESSABLE_MASK);
        self.masks.set_active(instrument, step, true);
    }

    /// Store a single field override and mark it as modified.
    pub fn set_field(&self, instrument: usize, step: usize, field: usize, value: f32) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT || field >= FIELD_COUNT {
            return;
        }
        self.values.set(instrument, step, field, value);
        self.field_masks.set(instrument, step, field);
        self.masks.set_active(instrument, step, true);
    }

    /// Clear the plock for a specific step/instrument.
    /// Drop ONE field's override, keeping the p-lock itself active — the
    /// per-row revert affordance ([184]). The step keeps following the lane's
    /// global value for that field (Link semantics).
    pub fn clear_field(&self, instrument: usize, step: usize, field: usize) {
        self.field_masks.clear(instrument, step, field);
    }

    pub fn clear(&self, instrument: usize, step: usize) {
        self.masks.set_active(instrument, step, false);
        self.field_masks.clear_all(instrument, step);
    }

    /// Clear every plock in the entire grid.
    /// Call before restore_from_buffers() so old plocks don't leak into the new pattern.
    pub fn clear_all(&self) {
        for inst in 0..INSTRUMENT_COUNT {
            self.masks.masks[inst].store(0, Ordering::Relaxed);
            for step in 0..STEP_COUNT {
                self.field_masks.clear_all(inst, step);
                for field in 0..FIELD_COUNT {
                    self.values.set(inst, step, field, 0.0);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct PersistentPlockState {
    pub state: Arc<PlockState>,
}

impl PersistentPlockState {
    pub fn new() -> Self {
        Self {
            state: PlockState::new(),
        }
    }
}

impl<'a> PersistentField<'a, Vec<u8>> for PersistentPlockState {
    fn set(&self, new_value: Vec<u8>) {
        // Binary format: instrument * step * field f32 values (little-endian u32),
        // followed by instrument u64 step masks (new) or u16 step masks (old),
        // followed by instrument * step u64 field masks.
        let expected_values = INSTRUMENT_COUNT * STEP_COUNT * FIELD_COUNT * 4;
        let legacy_expected_values = INSTRUMENT_COUNT * STEP_COUNT * LEGACY_FIELD_COUNT * 4;
        let expected_masks_old = INSTRUMENT_COUNT * 2; // u16 (legacy format)
        let expected_masks_new = INSTRUMENT_COUNT * 8; // u64 (current format)
        let expected_field_masks = INSTRUMENT_COUNT * STEP_COUNT * 8;

        // Detect format based on total size.
        let has_new_format = new_value.len()
            >= expected_values + expected_masks_new + expected_field_masks
            || (new_value.len()
                >= legacy_expected_values + expected_masks_new + expected_field_masks
                && new_value.len() < expected_values + expected_masks_old + expected_field_masks);

        let value_field_count = if new_value.len() >= expected_values + expected_masks_new {
            FIELD_COUNT
        } else if new_value.len() >= legacy_expected_values + expected_masks_new {
            LEGACY_FIELD_COUNT
        } else if new_value.len() >= expected_values + expected_masks_old {
            FIELD_COUNT
        } else if new_value.len() >= legacy_expected_values + expected_masks_old {
            LEGACY_FIELD_COUNT
        } else {
            return;
        };
        let values_len = INSTRUMENT_COUNT * STEP_COUNT * value_field_count * 4;
        let masks_size = if has_new_format {
            expected_masks_new
        } else {
            expected_masks_old
        };

        let mut offset = 0usize;
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                for field in 0..value_field_count {
                    let bytes = [
                        new_value[offset],
                        new_value[offset + 1],
                        new_value[offset + 2],
                        new_value[offset + 3],
                    ];
                    let val = f32::from_le_bytes(bytes);
                    self.state.values.set(inst, step, field, val);
                    offset += 4;
                }
            }
        }
        for inst in 0..INSTRUMENT_COUNT {
            if has_new_format {
                let bytes = [
                    new_value[offset],
                    new_value[offset + 1],
                    new_value[offset + 2],
                    new_value[offset + 3],
                    new_value[offset + 4],
                    new_value[offset + 5],
                    new_value[offset + 6],
                    new_value[offset + 7],
                ];
                let mask = u64::from_le_bytes(bytes);
                self.state.masks.masks[inst].store(mask, Ordering::Relaxed);
                offset += 8;
            } else {
                let bytes = [new_value[offset], new_value[offset + 1]];
                let mask = u16::from_le_bytes(bytes) as u64;
                self.state.masks.masks[inst].store(mask, Ordering::Relaxed);
                offset += 2;
            }
        }

        if new_value.len() >= values_len + masks_size + expected_field_masks {
            for inst in 0..INSTRUMENT_COUNT {
                for step in 0..STEP_COUNT {
                    let bytes = [
                        new_value[offset],
                        new_value[offset + 1],
                        new_value[offset + 2],
                        new_value[offset + 3],
                        new_value[offset + 4],
                        new_value[offset + 5],
                        new_value[offset + 6],
                        new_value[offset + 7],
                    ];
                    let mask = u64::from_le_bytes(bytes);
                    self.state.field_masks.set_raw(inst, step, mask);
                    offset += 8;
                }
            }
        } else {
            // Retro-compatibility: old presets had no field masks.
            // Treat every active plock as a full snapshot (all bits set).
            for inst in 0..INSTRUMENT_COUNT {
                for step in 0..STEP_COUNT {
                    if self.state.masks.is_active(inst, step) {
                        if value_field_count == LEGACY_FIELD_COUNT {
                            self.state.field_masks.set_legacy_snapshot(inst, step);
                        } else {
                            self.state.field_masks.set_all(inst, step);
                        }
                    }
                }
            }
        }
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&Vec<u8>) -> R,
    {
        let mut result = Vec::with_capacity(
            INSTRUMENT_COUNT * STEP_COUNT * FIELD_COUNT * 4
                + INSTRUMENT_COUNT * 8
                + INSTRUMENT_COUNT * STEP_COUNT * 8,
        );
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                for field in 0..FIELD_COUNT {
                    result
                        .extend_from_slice(&self.state.values.get(inst, step, field).to_le_bytes());
                }
            }
        }
        for inst in 0..INSTRUMENT_COUNT {
            result.extend_from_slice(
                &self.state.masks.masks[inst]
                    .load(Ordering::Relaxed)
                    .to_le_bytes(),
            );
        }
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                result.extend_from_slice(&self.state.field_masks.get_raw(inst, step).to_le_bytes());
            }
        }
        f(&result)
    }
}

// ============================================================================
// Sequencer Plocks — per-step sequencer parameters (probability, stutter, etc.)
// ============================================================================

/// Condition types for sequencer step parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepCondition {
    Always = 0,
    First = 1,    // First loop/bar only
    NotFirst = 2, // Not first loop/bar
    Half1 = 3,    // 1/2 — first half
    Half2 = 4,    // 2/2 — second half
    Third1 = 5,   // 1/3
    Third2 = 6,   // 2/3
    Third3 = 7,   // 3/3
    Fourth1 = 8,  // 1/4
    Fourth2 = 9,  // 2/4
    Fourth3 = 10, // 3/4
    Fourth4 = 11, // 4/4
}

impl Default for StepCondition {
    fn default() -> Self {
        StepCondition::Always
    }
}

impl StepCondition {
    pub fn label(&self) -> &'static str {
        match self {
            StepCondition::Always => "Always",
            StepCondition::First => "1st loop only",
            StepCondition::NotFirst => "Not 1st loop",
            StepCondition::Half1 => "1/2",
            StepCondition::Half2 => "2/2",
            StepCondition::Third1 => "1/3",
            StepCondition::Third2 => "2/3",
            StepCondition::Third3 => "3/3",
            StepCondition::Fourth1 => "1/4",
            StepCondition::Fourth2 => "2/4",
            StepCondition::Fourth3 => "3/4",
            StepCondition::Fourth4 => "4/4",
        }
    }

    pub fn all() -> &'static [StepCondition] {
        &[
            StepCondition::Always,
            StepCondition::First,
            StepCondition::NotFirst,
            StepCondition::Half1,
            StepCondition::Half2,
            StepCondition::Third1,
            StepCondition::Third2,
            StepCondition::Third3,
            StepCondition::Fourth1,
            StepCondition::Fourth2,
            StepCondition::Fourth3,
            StepCondition::Fourth4,
        ]
    }
}

/// Sequencer parameters for a single step × instrument.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SequencerStepParams {
    pub probability: f32,  // 0.0 - 1.0, default 1.0 = always trigger
    pub stutter_count: u8, // 1-16, default 1 = no stutter
    pub condition: StepCondition,
    pub microtiming_ms: f32, // -100.0 to +100.0, default 0.0
    /// Step-scoped solo: while this cell (or its fusion span) plays, every
    /// non-soloed lane is muted for those steps. Independent of the lane-level
    /// `S` tag. Default false.
    pub solo: bool,
}

impl Default for SequencerStepParams {
    fn default() -> Self {
        Self {
            probability: 1.0,
            stutter_count: 1,
            condition: StepCondition::Always,
            microtiming_ms: 0.0,
            solo: false,
        }
    }
}

/// Lock-free storage for sequencer step parameters.
/// Same pattern as PlockState: masks + per-cell values.
pub struct SequencerPlockState {
    pub masks: [AtomicU64; INSTRUMENT_COUNT],
    pub probabilities: Vec<Vec<AtomicU32>>,
    pub stutters: Vec<Vec<AtomicU32>>,
    pub conditions: Vec<Vec<AtomicU32>>,
    pub microtimings: Vec<Vec<AtomicU32>>,
    /// Per-instrument bitmask (1 bit per step) of step-scoped solos. Kept as a
    /// bitmask rather than a per-cell value because solo is boolean and this
    /// matches the compact `masks` representation.
    pub solo_masks: [AtomicU64; INSTRUMENT_COUNT],
}

impl SequencerPlockState {
    pub fn new() -> Self {
        Self {
            masks: std::array::from_fn(|_| AtomicU64::new(0)),
            solo_masks: std::array::from_fn(|_| AtomicU64::new(0)),
            probabilities: (0..INSTRUMENT_COUNT)
                .map(|_| {
                    (0..STEP_COUNT)
                        .map(|_| AtomicU32::new(f32::to_bits(1.0)))
                        .collect()
                })
                .collect(),
            stutters: (0..INSTRUMENT_COUNT)
                .map(|_| {
                    (0..STEP_COUNT)
                        .map(|_| AtomicU32::new(f32::to_bits(1.0)))
                        .collect()
                })
                .collect(),
            conditions: (0..INSTRUMENT_COUNT)
                .map(|_| (0..STEP_COUNT).map(|_| AtomicU32::new(0)).collect())
                .collect(),
            microtimings: (0..INSTRUMENT_COUNT)
                .map(|_| (0..STEP_COUNT).map(|_| AtomicU32::new(0)).collect())
                .collect(),
        }
    }

    pub fn is_active(&self, instrument: usize, step: usize) -> bool {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return false;
        }
        let mask = self.masks[instrument].load(Ordering::Acquire);
        (mask & (1u64 << step)) != 0
    }

    pub fn set_active(&self, instrument: usize, step: usize, active: bool) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        let mask = self.masks[instrument].load(Ordering::Relaxed);
        let new_mask = if active {
            mask | (1u64 << step)
        } else {
            mask & !(1u64 << step)
        };
        self.masks[instrument].store(new_mask, Ordering::Release);
    }

    pub fn get(&self, instrument: usize, step: usize) -> Option<SequencerStepParams> {
        if !self.is_active(instrument, step) {
            return None;
        }
        Some(SequencerStepParams {
            probability: f32::from_bits(
                self.probabilities[instrument][step].load(Ordering::Acquire),
            ),
            stutter_count: f32::from_bits(self.stutters[instrument][step].load(Ordering::Acquire))
                as u8,
            condition: match self.conditions[instrument][step].load(Ordering::Acquire) {
                1 => StepCondition::First,
                2 => StepCondition::NotFirst,
                3 => StepCondition::Half1,
                4 => StepCondition::Half2,
                5 => StepCondition::Third1,
                6 => StepCondition::Third2,
                7 => StepCondition::Third3,
                8 => StepCondition::Fourth1,
                9 => StepCondition::Fourth2,
                10 => StepCondition::Fourth3,
                11 => StepCondition::Fourth4,
                _ => StepCondition::Always,
            },
            microtiming_ms: f32::from_bits(
                self.microtimings[instrument][step].load(Ordering::Acquire),
            ),
            solo: self.is_solo(instrument, step),
        })
    }

    pub fn set(&self, instrument: usize, step: usize, params: &SequencerStepParams) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.probabilities[instrument][step].store(params.probability.to_bits(), Ordering::Release);
        self.stutters[instrument][step]
            .store((params.stutter_count as f32).to_bits(), Ordering::Release);
        self.conditions[instrument][step].store(params.condition as u32, Ordering::Release);
        self.microtimings[instrument][step]
            .store(params.microtiming_ms.to_bits(), Ordering::Release);
        self.set_solo(instrument, step, params.solo);
        self.set_active(instrument, step, true);
    }

    /// Whether the cell at (instrument, step) has step-solo enabled.
    pub fn is_solo(&self, instrument: usize, step: usize) -> bool {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return false;
        }
        (self.solo_masks[instrument].load(Ordering::Acquire) & (1u64 << step)) != 0
    }

    /// Set/clear the step-solo bit. Enabling it also marks the cell as an active
    /// seq-plock so it persists and renders in the sequencer p-lock layer.
    pub fn set_solo(&self, instrument: usize, step: usize, solo: bool) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        let mask = self.solo_masks[instrument].load(Ordering::Relaxed);
        let new_mask = if solo {
            mask | (1u64 << step)
        } else {
            mask & !(1u64 << step)
        };
        self.solo_masks[instrument].store(new_mask, Ordering::Release);
        if solo {
            self.set_active(instrument, step, true);
        }
    }

    pub fn set_probability(&self, instrument: usize, step: usize, value: f32) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.probabilities[instrument][step].store(value.to_bits(), Ordering::Release);
        self.set_active(instrument, step, true);
    }

    pub fn set_stutter(&self, instrument: usize, step: usize, value: u8) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.stutters[instrument][step].store((value as f32).to_bits(), Ordering::Release);
        self.set_active(instrument, step, true);
    }

    pub fn set_condition(&self, instrument: usize, step: usize, value: StepCondition) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.conditions[instrument][step].store(value as u32, Ordering::Release);
        self.set_active(instrument, step, true);
    }

    pub fn set_microtiming(&self, instrument: usize, step: usize, value: f32) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.microtimings[instrument][step].store(value.to_bits(), Ordering::Release);
        self.set_active(instrument, step, true);
    }

    pub fn clear(&self, instrument: usize, step: usize) {
        self.set_solo(instrument, step, false);
        self.set_active(instrument, step, false);
    }

    /// Clear every sequencer plock in the entire grid.
    pub fn clear_all(&self) {
        for inst in 0..INSTRUMENT_COUNT {
            self.masks[inst].store(0, Ordering::Relaxed);
            self.solo_masks[inst].store(0, Ordering::Relaxed);
            for step in 0..STEP_COUNT {
                self.probabilities[inst][step].store(f32::to_bits(1.0), Ordering::Relaxed);
                self.stutters[inst][step].store(f32::to_bits(1.0), Ordering::Relaxed);
                self.conditions[inst][step].store(0, Ordering::Relaxed);
                self.microtimings[inst][step].store(0, Ordering::Relaxed);
            }
        }
    }

    /// Union of the step-solo windows across all instruments, as a 64-bit mask
    /// (bit S set = step S is inside some soloed cell's span). A normal soloed
    /// cell covers its own step; a soloed fused cell covers its whole span, so
    /// the mute of the other lanes lasts exactly as long as the playhead sits on
    /// the soloed cell. `fusion_span_len(inst, start)` returns the number of
    /// steps the cell starting at `start` occupies (1 if not a fusion start).
    ///
    /// Cheap enough to recompute per audio block (bit tests, no allocation).
    pub fn solo_window(&self, mut fusion_span_len: impl FnMut(usize, usize) -> usize) -> u64 {
        let mut window = 0u64;
        for inst in 0..INSTRUMENT_COUNT {
            let solos = self.solo_masks[inst].load(Ordering::Acquire);
            if solos == 0 {
                continue;
            }
            for step in 0..STEP_COUNT {
                if (solos & (1u64 << step)) == 0 {
                    continue;
                }
                let span = fusion_span_len(inst, step).max(1);
                for s in step..(step + span).min(STEP_COUNT) {
                    window |= 1u64 << s;
                }
            }
        }
        window
    }
}

#[derive(Clone)]
pub struct PersistentSequencerPlockState {
    pub state: Arc<SequencerPlockState>,
}

impl PersistentSequencerPlockState {
    pub fn new() -> Self {
        Self {
            state: Arc::new(SequencerPlockState::new()),
        }
    }
}

impl<'a> PersistentField<'a, Vec<u8>> for PersistentSequencerPlockState {
    fn set(&self, new_value: Vec<u8>) {
        let cell_count = INSTRUMENT_COUNT * STEP_COUNT;
        let expected_size = cell_count * 4 * 4 + INSTRUMENT_COUNT * 8;

        if new_value.len() < expected_size {
            return;
        }

        let mut offset = 0;
        let read_f32 = |bytes: &[u8], idx: &mut usize| -> f32 {
            let val = f32::from_le_bytes([
                bytes[*idx],
                bytes[*idx + 1],
                bytes[*idx + 2],
                bytes[*idx + 3],
            ]);
            *idx += 4;
            val
        };

        let read_u32 = |bytes: &[u8], idx: &mut usize| -> u32 {
            let val = u32::from_le_bytes([
                bytes[*idx],
                bytes[*idx + 1],
                bytes[*idx + 2],
                bytes[*idx + 3],
            ]);
            *idx += 4;
            val
        };

        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                let prob = read_f32(&new_value, &mut offset);
                let stutter = read_f32(&new_value, &mut offset) as u8;
                let condition = match read_u32(&new_value, &mut offset) {
                    1 => StepCondition::First,
                    2 => StepCondition::NotFirst,
                    3 => StepCondition::Half1,
                    4 => StepCondition::Half2,
                    5 => StepCondition::Third1,
                    6 => StepCondition::Third2,
                    7 => StepCondition::Third3,
                    8 => StepCondition::Fourth1,
                    9 => StepCondition::Fourth2,
                    10 => StepCondition::Fourth3,
                    11 => StepCondition::Fourth4,
                    _ => StepCondition::Always,
                };
                let micro = read_f32(&new_value, &mut offset);

                self.state.probabilities[inst][step].store(prob.to_bits(), Ordering::Relaxed);
                self.state.stutters[inst][step]
                    .store((stutter as f32).to_bits(), Ordering::Relaxed);
                self.state.conditions[inst][step].store(condition as u32, Ordering::Relaxed);
                self.state.microtimings[inst][step].store(micro.to_bits(), Ordering::Relaxed);
            }
        }

        for inst in 0..INSTRUMENT_COUNT {
            let mask = u64::from_le_bytes([
                new_value[offset],
                new_value[offset + 1],
                new_value[offset + 2],
                new_value[offset + 3],
                new_value[offset + 4],
                new_value[offset + 5],
                new_value[offset + 6],
                new_value[offset + 7],
            ]);
            offset += 8;
            self.state.masks[inst].store(mask, Ordering::Relaxed);
        }
    }

    fn map<F, T>(&self, f: F) -> T
    where
        F: Fn(&Vec<u8>) -> T,
    {
        let mut result =
            Vec::with_capacity(INSTRUMENT_COUNT * STEP_COUNT * 4 * 4 + INSTRUMENT_COUNT * 8);

        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                result.extend_from_slice(
                    &f32::from_bits(self.state.probabilities[inst][step].load(Ordering::Relaxed))
                        .to_le_bytes(),
                );
                result.extend_from_slice(
                    &f32::from_bits(self.state.stutters[inst][step].load(Ordering::Relaxed))
                        .to_le_bytes(),
                );
                result.extend_from_slice(
                    &self.state.conditions[inst][step]
                        .load(Ordering::Relaxed)
                        .to_le_bytes(),
                );
                result.extend_from_slice(
                    &f32::from_bits(self.state.microtimings[inst][step].load(Ordering::Relaxed))
                        .to_le_bytes(),
                );
            }
        }

        for inst in 0..INSTRUMENT_COUNT {
            result.extend_from_slice(&self.state.masks[inst].load(Ordering::Relaxed).to_le_bytes());
        }

        f(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_settings() -> VoiceSettings {
        VoiceSettings {
            frequency: 100.0,
            decay: 0.2,
            volume: 0.8,
            filter_freq: 1000.0,
            attack: 0.0015,
            release: 0.1,
            decay_curve: 5.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 1.0,
            stereo: 0.0,
            algo: 1,
            special: [0.0; 32],
        }
    }

    #[test]
    fn clap_echo_roundtrips_as_special_field() {
        let state = PlockState::new();
        let mut settings = base_settings();
        settings.special[0] = 2.5;

        state.set_settings(7, 3, &settings);
        let global = base_settings();
        let restored = state
            .get_settings(7, 3, &global)
            .expect("plock should exist");

        assert_eq!(restored.special[0], 2.5);
        assert_eq!(state.values.get(7, 3, SPECIAL_FIELD_START), 2.5);
        assert_eq!(state.values.get(7, 3, LEGACY_CLAP_ECHO_FIELD), 0.0);
    }

    /// [187] The special that Attack used to shadow is p-lockable again, on the
    /// reserved slot. This is the end-to-end proof: it survives `set_settings` and
    /// comes back out of `get_settings`, and it does NOT disturb Attack.
    #[test]
    fn the_rehomed_special_survives_a_settings_roundtrip() {
        let state = PlockState::new();
        let mut settings = base_settings();
        settings.special[4] = 0.42;
        settings.attack = 0.007;

        state.set_settings(0, 1, &settings);
        let restored = state
            .get_settings(0, 1, &base_settings())
            .expect("plock should exist");

        assert_eq!(restored.special[4], 0.42, "the re-homed special round-trips");
        assert_eq!(restored.attack, 0.007, "Attack is untouched by it");
        assert_eq!(
            state.values.get(0, 1, crate::param_id::SPECIAL_4_FIELD),
            0.42,
            "stored on the reserved slot, not on Attack's field"
        );
        assert_eq!(state.values.get(0, 1, ATTACK_FIELD), 0.007);
    }

    /// [187] chose the reserved slot 45 precisely so the dead legacy clap-echo
    /// field 12 could stay untouched. This pins that: the Clap's own fallback is
    /// unaffected by the re-homing.
    #[test]
    fn the_rehoming_leaves_the_clap_legacy_field_alone() {
        let state = PlockState::new();
        // An old blob: echo in field 12, special[0] empty, everything masked.
        state.values.set(7, 5, LEGACY_CLAP_ECHO_FIELD, 1.25);
        state.values.set(7, 5, SPECIAL_FIELD_START, 0.0);
        state.masks.set_active(7, 5, true);
        state.field_masks.set_all(7, 5);

        let restored = state
            .get_settings(7, 5, &base_settings())
            .expect("plock should exist");
        assert_eq!(
            restored.special[0], 1.25,
            "the Clap still reads its legacy echo from field 12"
        );
        // And field 12 is still claimed by nobody as a parameter.
        assert_eq!(crate::param_id::ParamId::from_plock_field(LEGACY_CLAP_ECHO_FIELD), None);
    }

    #[test]
    fn legacy_clap_echo_field_still_loads() {
        let state = PlockState::new();
        state.values.set(7, 3, LEGACY_CLAP_ECHO_FIELD, 1.75);
        state.values.set(7, 3, SPECIAL_FIELD_START, 0.0);
        state.masks.set_active(7, 3, true);
        // Simulate retro-compatibility: old presets have all field bits set
        state.field_masks.set_all(7, 3);

        let global = base_settings();
        let restored = state
            .get_settings(7, 3, &global)
            .expect("plock should exist");

        assert_eq!(restored.special[0], 1.75);
    }

    #[test]
    fn b8_specials_roundtrip() {
        let state = PlockState::new();
        let mut settings = base_settings();
        settings.special[0] = 1.1;
        settings.special[1] = 1.2;
        settings.special[2] = 1.3;
        settings.special[3] = 7000.0;

        state.set_settings(11, 4, &settings);
        let global = base_settings();
        let restored = state
            .get_settings(11, 4, &global)
            .expect("plock should exist");

        assert_eq!(restored.special[0], 1.1);
        assert_eq!(restored.special[1], 1.2);
        assert_eq!(restored.special[2], 1.3);
        assert_eq!(restored.special[3], 7000.0);
    }

    #[test]
    fn perc1_specials_and_algo_roundtrip() {
        let state = PlockState::new();
        let mut settings = base_settings();
        settings.algo = 1;
        settings.special[0] = -0.5;
        settings.special[1] = 120.0;
        settings.special[2] = 0.6;
        settings.special[3] = 0.7;

        state.set_settings(12, 5, &settings);
        let global = base_settings();
        let restored = state
            .get_settings(12, 5, &global)
            .expect("plock should exist");

        assert_eq!(restored.algo, 1);
        assert_eq!(restored.special[0], -0.5);
        assert_eq!(restored.special[1], 120.0);
        assert_eq!(restored.special[2], 0.6);
        assert_eq!(restored.special[3], 0.7);
    }

    #[test]
    fn attack_roundtrips_as_appended_field() {
        let state = PlockState::new();
        let mut settings = base_settings();
        settings.attack = 0.045;

        state.set_settings(0, 2, &settings);
        let global = base_settings();
        let restored = state
            .get_settings(0, 2, &global)
            .expect("plock should exist");

        assert_eq!(restored.attack, 0.045);
        assert_eq!(state.values.get(0, 2, ATTACK_FIELD), 0.045);
        assert_eq!(state.values.get(0, 2, LEGACY_CLAP_ECHO_FIELD), 0.0);
    }

    #[test]
    fn link_mode_returns_global_when_mask_empty() {
        let state = PlockState::new();
        state.masks.set_active(0, 0, true);
        // field_masks left at 0 → link mode

        let global = base_settings();
        let restored = state
            .get_settings(0, 0, &global)
            .expect("plock should exist");

        assert_eq!(restored.frequency, global.frequency);
        assert_eq!(restored.decay, global.decay);
        assert_eq!(restored.volume, global.volume);
    }

    #[test]
    fn merge_takes_modified_fields_from_plock() {
        let state = PlockState::new();
        let global = base_settings();

        state.set_field(0, 0, 1, 0.99); // decay
        state.set_field(0, 0, 2, 0.42); // volume

        let restored = state
            .get_settings(0, 0, &global)
            .expect("plock should exist");

        assert_eq!(restored.frequency, global.frequency); // unchanged
        assert_eq!(restored.decay, 0.99);
        assert_eq!(restored.volume, 0.42);
        assert_eq!(restored.filter_freq, global.filter_freq); // unchanged
    }

    #[test]
    fn set_field_only_sets_one_bit() {
        let state = PlockState::new();
        state.set_field(0, 0, 3, 5000.0);

        assert!(state.field_masks.is_set(0, 0, 3));
        assert!(!state.field_masks.is_set(0, 0, 0));
        assert!(!state.field_masks.is_set(0, 0, 1));
        assert!(!state.field_masks.is_set(0, 0, 2));
    }

    #[test]
    fn clear_removes_field_mask() {
        let state = PlockState::new();
        state.set_field(0, 0, 1, 0.5);
        assert!(state.masks.is_active(0, 0));
        assert!(state.field_masks.is_set(0, 0, 1));

        state.clear(0, 0);
        assert!(!state.masks.is_active(0, 0));
        assert_eq!(state.field_masks.get(0, 0), 0);
    }

    #[test]
    fn clear_field_unlinks_without_clearing_plock() {
        let state = PlockState::new();
        let global = base_settings();

        state.set_field(0, 0, 1, 0.99); // override decay
        assert!(state.field_masks.is_set(0, 0, 1));

        state.field_masks.clear(0, 0, 1);
        assert!(!state.field_masks.is_set(0, 0, 1));
        assert!(state.masks.is_active(0, 0)); // plock still active

        let restored = state
            .get_settings(0, 0, &global)
            .expect("plock should exist");
        assert_eq!(restored.decay, global.decay); // falls back to global
    }

    #[test]
    fn plock_supports_steps_16_to_63() {
        let state = PlockState::new();
        let global = base_settings();

        // Set plocks at boundaries beyond u16 range
        state.set_field(0, 16, 1, 0.99);
        state.set_field(0, 31, 2, 0.42);
        state.set_field(0, 63, 3, 5000.0);

        assert!(state.masks.is_active(0, 16));
        assert!(state.masks.is_active(0, 31));
        assert!(state.masks.is_active(0, 63));

        let r16 = state
            .get_settings(0, 16, &global)
            .expect("plock at step 16");
        assert_eq!(r16.decay, 0.99);

        let r31 = state
            .get_settings(0, 31, &global)
            .expect("plock at step 31");
        assert_eq!(r31.volume, 0.42);

        let r63 = state
            .get_settings(0, 63, &global)
            .expect("plock at step 63");
        assert_eq!(r63.filter_freq, 5000.0);
    }

    #[test]
    fn plock_persistence_roundtrips_step_63() {
        let state = PlockState::new();
        let mut settings = base_settings();
        settings.decay = 0.99;
        settings.volume = 0.42;
        settings.filter_freq = 5000.0;

        state.set_settings(0, 16, &settings);
        state.set_settings(1, 31, &settings);
        state.set_settings(2, 63, &settings);

        // Serialize from the populated state and restore into a fresh one.
        let persistent_src = PersistentPlockState { state };
        let serialized = persistent_src.map(|v| v.clone());

        let persistent_dst = PersistentPlockState::new();
        persistent_dst.set(serialized);

        let global = base_settings();
        assert!(persistent_dst.state.masks.is_active(0, 16));
        assert!(persistent_dst.state.masks.is_active(1, 31));
        assert!(persistent_dst.state.masks.is_active(2, 63));

        let r16 = persistent_dst
            .state
            .get_settings(0, 16, &global)
            .expect("restored step 16");
        assert_eq!(r16.decay, 0.99);

        let r31 = persistent_dst
            .state
            .get_settings(1, 31, &global)
            .expect("restored step 31");
        assert_eq!(r31.volume, 0.42);

        let r63 = persistent_dst
            .state
            .get_settings(2, 63, &global)
            .expect("restored step 63");
        assert_eq!(r63.filter_freq, 5000.0);
    }

    #[test]
    fn sequencer_step_params_default_is_playable() {
        let params = SequencerStepParams::default();

        assert_eq!(params.probability, 1.0);
        assert_eq!(params.stutter_count, 1);
        assert_eq!(params.condition, StepCondition::Always);
        assert_eq!(params.microtiming_ms, 0.0);
    }

    #[test]
    fn sequencer_condition_setter_roundtrips() {
        let state = SequencerPlockState::new();

        state.set_condition(0, 7, StepCondition::NotFirst);

        let params = state.get(0, 7).expect("sequencer plock should exist");
        assert_eq!(params.condition, StepCondition::NotFirst);
        assert_eq!(params.probability, 1.0);
        assert_eq!(params.stutter_count, 1);
    }

    #[test]
    fn set_solo_marks_active_and_roundtrips() {
        let state = SequencerPlockState::new();
        assert!(!state.is_solo(0, 3));

        state.set_solo(0, 3, true);
        assert!(state.is_solo(0, 3));
        // Enabling solo also makes the cell an active seq-plock.
        assert!(state.is_active(0, 3));
        let params = state.get(0, 3).expect("sequencer plock should exist");
        assert!(params.solo);

        state.set_solo(0, 3, false);
        assert!(!state.is_solo(0, 3));
    }

    #[test]
    fn solo_window_covers_step_and_fusion_span() {
        let state = SequencerPlockState::new();
        state.set_solo(0, 2, true); // normal cell → covers only step 2
        state.set_solo(1, 5, true); // fused cell → covers steps 5,6,7

        let window = state.solo_window(|inst, start| {
            if inst == 1 && start == 5 {
                3
            } else {
                1
            }
        });

        assert_ne!(window & (1 << 2), 0); // normal solo
        assert_ne!(window & (1 << 5), 0);
        assert_ne!(window & (1 << 6), 0); // fusion span
        assert_ne!(window & (1 << 7), 0);
        assert_eq!(window & (1 << 3), 0); // untouched step
        assert_eq!(window & (1 << 8), 0); // just past the fusion span

        // No solos → empty window.
        assert_eq!(SequencerPlockState::new().solo_window(|_, _| 1), 0);
    }

    #[test]
    fn clear_resets_solo() {
        let state = SequencerPlockState::new();
        state.set_solo(2, 4, true);
        assert!(state.is_solo(2, 4));
        state.clear(2, 4);
        assert!(!state.is_solo(2, 4));
    }
}
