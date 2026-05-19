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
            algo: v.get(instrument, step, 13) as u8,
            special: {
                let mut s = [0.0f32; 8];
                s[0] = v.get(instrument, step, 14);
                s[1] = v.get(instrument, step, 15);
                s[2] = v.get(instrument, step, 16);
                s[3] = v.get(instrument, step, 17);
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
        v.set(instrument, step, 13, settings.algo as f32);
        v.set(instrument, step, 14, settings.special[0]);
        v.set(instrument, step, 15, settings.special[1]);
        v.set(instrument, step, 16, settings.special[2]);
        v.set(instrument, step, 17, settings.special[3]);
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
        // Binary format: 11 * 16 * 12 f32 values (little-endian u32) followed by
        // 11 * 2 bytes for u16 masks.
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
