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
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::Arc;

use crate::sequencer::pattern::INSTRUMENT_COUNT;
use crate::synthesis::VoiceSettings;

pub const STEP_COUNT: usize = 16;
pub const FIELD_COUNT: usize = 46;  // 13 standard + 1 algo + 32 special
const LEGACY_FIELD_COUNT: usize = 18;
const LEGACY_CLAP_ECHO_FIELD: usize = 12;
const ALGO_FIELD: usize = 13;
pub const SPECIAL_FIELD_START: usize = 14;
pub const SPECIAL_FIELD_COUNT: usize = 32;
const ATTACK_FIELD: usize = 18;

/// Active-bit mask: one u16 per instrument (bit = step has a plock).
/// Stored as atomic so the UI can toggle bits without locking.
pub struct PlockMasks {
    pub masks: [AtomicU16; INSTRUMENT_COUNT],
}

impl PlockMasks {
    pub fn new() -> Self {
        Self {
            masks: std::array::from_fn(|_| AtomicU16::new(0)),
        }
    }

    pub fn is_active(&self, instrument: usize, step: usize) -> bool {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return false;
        }
        let mask = self.masks[instrument].load(Ordering::Relaxed);
        (mask & (1u16 << step)) != 0
    }

    pub fn set_active(&self, instrument: usize, step: usize, active: bool) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        let mask = self.masks[instrument].load(Ordering::Relaxed);
        let new_mask = if active {
            mask | (1u16 << step)
        } else {
            mask & !(1u16 << step)
        };
        self.masks[instrument].store(new_mask, Ordering::Relaxed);
    }
}

/// Lock-free storage for plock values.
/// `values[instrument][step][field]` is a f32 bitcast.
/// Only meaningful when the corresponding bit in `PlockMasks` is set.
pub struct PlockValues {
    pub values: [[[AtomicU64; FIELD_COUNT]; STEP_COUNT]; INSTRUMENT_COUNT],
}

impl PlockValues {
    pub fn new() -> Self {
        Self {
            values: std::array::from_fn(|_| {
                std::array::from_fn(|_| std::array::from_fn(|_| AtomicU64::new(0)))
            }),
        }
    }

    pub fn get(&self, instrument: usize, step: usize, field: usize) -> f32 {
        f32::from_bits(self.values[instrument][step][field].load(Ordering::Relaxed) as u32)
    }

    pub fn set(&self, instrument: usize, step: usize, field: usize, value: f32) {
        self.values[instrument][step][field].store(value.to_bits() as u64, Ordering::Relaxed);
    }
}

/// Per-field modification mask: one u32 per instrument × step.
/// Bit `1 << field` is set when that field has been explicitly overridden.
pub struct PlockFieldMasks {
    masks: [[AtomicU64; STEP_COUNT]; INSTRUMENT_COUNT],
}

impl PlockFieldMasks {
    pub fn new() -> Self {
        Self {
            masks: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU64::new(0))),
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

    pub fn set_raw(&self, instrument: usize, step: usize, mask: u64) {
        if instrument >= INSTRUMENT_COUNT || step >= STEP_COUNT {
            return;
        }
        self.masks[instrument][step].store(mask, Ordering::Relaxed);
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
            let field = SPECIAL_FIELD_START + i;
            // Skip attack field — it's already read above
            if field == ATTACK_FIELD {
                continue;
            }
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
            let field = SPECIAL_FIELD_START + index;
            // Skip attack field to avoid overwriting it with special params
            if field == ATTACK_FIELD {
                continue;
            }
            v.set(
                instrument,
                step,
                field,
                settings.special[index],
            );
        }
        self.field_masks.set_all(instrument, step);
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
    pub fn clear(&self, instrument: usize, step: usize) {
        self.masks.set_active(instrument, step, false);
        self.field_masks.clear_all(instrument, step);
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
        // followed by instrument u16 step masks, followed by instrument * step u32 field masks.
        let expected_values = INSTRUMENT_COUNT * STEP_COUNT * FIELD_COUNT * 4;
        let legacy_expected_values = INSTRUMENT_COUNT * STEP_COUNT * LEGACY_FIELD_COUNT * 4;
        let expected_masks = INSTRUMENT_COUNT * 2;
        let expected_field_masks = INSTRUMENT_COUNT * STEP_COUNT * 8;
        let value_field_count = if new_value.len() >= expected_values + expected_masks {
            FIELD_COUNT
        } else if new_value.len() >= legacy_expected_values + expected_masks {
            LEGACY_FIELD_COUNT
        } else {
            return;
        };
        let values_len = INSTRUMENT_COUNT * STEP_COUNT * value_field_count * 4;

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
            let bytes = [new_value[offset], new_value[offset + 1]];
            let mask = u16::from_le_bytes(bytes);
            self.state.masks.masks[inst].store(mask, Ordering::Relaxed);
            offset += 2;
        }

        if new_value.len() >= values_len + expected_masks + expected_field_masks {
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
                + INSTRUMENT_COUNT * 2
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
        let restored = state.get_settings(0, 2, &global).expect("plock should exist");

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
}
