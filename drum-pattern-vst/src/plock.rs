//! Parameter locks (plocks) — per-step instrument sound overrides.
//!
//! Each instrument × step can store a complete override of the 12 sound settings
//! fields. When a step triggers, the audio thread applies the plock settings to
//! the voice before calling `trigger()`. When no plock is present, the voice
//! falls back to the global instrument settings.

use nih_plug::params::persist::PersistentField;
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::Arc;

use crate::sequencer::pattern::INSTRUMENT_COUNT;
use crate::synthesis::VoiceSettings;

pub const STEP_COUNT: usize = 16;
pub const FIELD_COUNT: usize = 18;
const LEGACY_CLAP_ECHO_FIELD: usize = 12;
const ALGO_FIELD: usize = 13;
pub const SPECIAL_FIELD_START: usize = 14;
pub const SPECIAL_FIELD_COUNT: usize = FIELD_COUNT - SPECIAL_FIELD_START;

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
    pub values: [[[AtomicU32; FIELD_COUNT]; STEP_COUNT]; INSTRUMENT_COUNT],
}

impl PlockValues {
    pub fn new() -> Self {
        Self {
            values: std::array::from_fn(|_| {
                std::array::from_fn(|_| {
                    std::array::from_fn(|_| AtomicU32::new(0))
                })
            }),
        }
    }

    pub fn get(&self, instrument: usize, step: usize, field: usize) -> f32 {
        f32::from_bits(self.values[instrument][step][field].load(Ordering::Relaxed))
    }

    pub fn set(&self, instrument: usize, step: usize, field: usize, value: f32) {
        self.values[instrument][step][field].store(value.to_bits(), Ordering::Relaxed);
    }
}

/// Combined plock state exposed to UI and audio thread.
pub struct PlockState {
    pub masks: PlockMasks,
    pub values: PlockValues,
}

impl PlockState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            masks: PlockMasks::new(),
            values: PlockValues::new(),
        })
    }

    /// Retrieve a complete VoiceSettings override if the step has a plock.
    pub fn get_settings(&self, instrument: usize, step: usize) -> Option<VoiceSettings> {
        if !self.masks.is_active(instrument, step) {
            return None;
        }
        let v = &self.values;
        Some(VoiceSettings {
            frequency: v.get(instrument, step, 0),
            decay: v.get(instrument, step, 1),
            volume: v.get(instrument, step, 2),
            filter_freq: v.get(instrument, step, 3),
            release: v.get(instrument, step, 4),
            decay_curve: v.get(instrument, step, 5),
            release_curve: v.get(instrument, step, 6),
            hold: v.get(instrument, step, 7),
            filter_env_amount: v.get(instrument, step, 8),
            filter_env_decay: v.get(instrument, step, 9),
            analog: v.get(instrument, step, 10),
            stereo: v.get(instrument, step, 11),
            algo: v.get(instrument, step, ALGO_FIELD) as u8,
            special: {
                let mut s = [0.0f32; 8];
                for index in 0..SPECIAL_FIELD_COUNT.min(s.len()) {
                    s[index] = v.get(instrument, step, SPECIAL_FIELD_START + index);
                }
                if instrument == 7 && s[0] == 0.0 {
                    let legacy_clap_echo = v.get(instrument, step, LEGACY_CLAP_ECHO_FIELD);
                    if legacy_clap_echo != 0.0 {
                        s[0] = legacy_clap_echo;
                    }
                }
                s
            },
        })
    }

    /// Store a complete VoiceSettings override for a step/instrument.
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
        if instrument == 7 {
            v.set(instrument, step, LEGACY_CLAP_ECHO_FIELD, 0.0);
        }
        v.set(instrument, step, ALGO_FIELD, settings.algo as f32);
        for index in 0..SPECIAL_FIELD_COUNT.min(settings.special.len()) {
            v.set(
                instrument,
                step,
                SPECIAL_FIELD_START + index,
                settings.special[index],
            );
        }
        self.masks.set_active(instrument, step, true);
    }

    /// Clear the plock for a specific step/instrument.
    pub fn clear(&self, instrument: usize, step: usize) {
        self.masks.set_active(instrument, step, false);
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
        // followed by instrument u16 masks. Field 12 is legacy Clap Echo, field
        // 13 is algo, fields 14..17 are special params.
        let expected_values = INSTRUMENT_COUNT * STEP_COUNT * FIELD_COUNT * 4;
        let expected_masks = INSTRUMENT_COUNT * 2;
        if new_value.len() < expected_values + expected_masks {
            return;
        }

        let mut offset = 0usize;
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                for field in 0..FIELD_COUNT {
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
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&Vec<u8>) -> R,
    {
        let mut result = Vec::with_capacity(
            INSTRUMENT_COUNT * STEP_COUNT * FIELD_COUNT * 4 + INSTRUMENT_COUNT * 2,
        );
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                for field in 0..FIELD_COUNT {
                    result.extend_from_slice(
                        &self.state.values.get(inst, step, field).to_le_bytes(),
                    );
                }
            }
        }
        for inst in 0..INSTRUMENT_COUNT {
            result.extend_from_slice(&self.state.masks.masks[inst].load(Ordering::Relaxed).to_le_bytes());
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
            release: 0.1,
            decay_curve: 5.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 1.0,
            stereo: 0.0,
            algo: 1,
            special: [0.0; 8],
        }
    }

    #[test]
    fn clap_echo_roundtrips_as_special_field() {
        let state = PlockState::new();
        let mut settings = base_settings();
        settings.special[0] = 2.5;

        state.set_settings(7, 3, &settings);
        let restored = state.get_settings(7, 3).expect("plock should exist");

        assert_eq!(restored.special[0], 2.5);
        assert_eq!(state.values.get(7, 3, SPECIAL_FIELD_START), 2.5);
        assert_eq!(state.values.get(7, 3, LEGACY_CLAP_ECHO_FIELD), 0.0);
    }

    #[test]
    fn legacy_clap_echo_field_still_loads() {
        let state = PlockState::new();
        state.values.set(7, 3, LEGACY_CLAP_ECHO_FIELD, 1.75);
        state.masks.set_active(7, 3, true);

        let restored = state.get_settings(7, 3).expect("plock should exist");

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
        let restored = state.get_settings(11, 4).expect("plock should exist");

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
        let restored = state.get_settings(12, 5).expect("plock should exist");

        assert_eq!(restored.algo, 1);
        assert_eq!(restored.special[0], -0.5);
        assert_eq!(restored.special[1], 120.0);
        assert_eq!(restored.special[2], 0.6);
        assert_eq!(restored.special[3], 0.7);
    }
}
