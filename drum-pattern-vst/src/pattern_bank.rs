//! Pattern bank — stores up to 8 pattern slots.
//!
//! Kept simple: each slot stores raw serialized bytes that can be
//! directly fed back into the plock / seq-plock / grid systems.

use crate::sequencer::pattern::STEP_COUNT;

pub const SLOT_COUNT: usize = 8;

/// Max serialized size for sound plock state.
/// Uses actual constants so the buffer never under-allocates when FIELD_COUNT grows.
pub const MAX_PLOCK_BYTES: usize = crate::sequencer::pattern::INSTRUMENT_COUNT
        * crate::sequencer::pattern::STEP_COUNT
        * crate::plock::FIELD_COUNT
        * 4 // values
    + crate::sequencer::pattern::INSTRUMENT_COUNT * 8 // masks
    + crate::sequencer::pattern::INSTRUMENT_COUNT
        * crate::sequencer::pattern::STEP_COUNT
        * 8; // field_masks

/// Max serialized size for sequencer plock state.
pub const MAX_SEQ_PLOCK_BYTES: usize = crate::sequencer::pattern::INSTRUMENT_COUNT
        * crate::sequencer::pattern::STEP_COUNT
        * 4
        * 4 // 4 fields per cell
    + crate::sequencer::pattern::INSTRUMENT_COUNT * 8; // masks

/// A single saved pattern slot.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternSlot {
    /// Step masks for all 64 steps.
    #[serde(with = "serde_arrays")]
    pub step_masks: [u16; STEP_COUNT],
    /// Serialized sound plock state.
    pub plock_bytes: Vec<u8>,
    /// Serialized sequencer plock state.
    pub seq_plock_bytes: Vec<u8>,
    /// Pattern length.
    pub pattern_length: u8,
    /// Occupied flag.
    pub occupied: bool,
}

impl Default for PatternSlot {
    fn default() -> Self {
        Self {
            step_masks: [0; STEP_COUNT],
            // Pre-allocate so capture() never allocates in the audio thread.
            plock_bytes: Vec::with_capacity(MAX_PLOCK_BYTES),
            seq_plock_bytes: Vec::with_capacity(MAX_SEQ_PLOCK_BYTES),
            pattern_length: 16,
            occupied: false,
        }
    }
}

/// A song sequence — an ordered chain of pattern slots.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SongSequence {
    /// Steps reference pattern slots (0-7 = P1-P8, -1 = empty/end).
    #[serde(with = "serde_arrays")]
    pub steps: [i8; 64],
    /// Number of active steps (1-64).
    pub length: u8,
    /// Loop the song when reaching the end.
    pub loop_enabled: bool,
}

impl Default for SongSequence {
    fn default() -> Self {
        Self {
            steps: [-1; 64],
            length: 0,
            loop_enabled: true,
        }
    }
}

impl SongSequence {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the slot index for a given song step, or None if empty/end.
    pub fn slot_at(&self, step: usize) -> Option<usize> {
        if step >= self.length as usize {
            return None;
        }
        let slot = self.steps[step];
        if slot < 0 || slot as usize >= SLOT_COUNT {
            None
        } else {
            Some(slot as usize)
        }
    }

    /// Set a step to a given slot index (-1 to clear).
    pub fn set_step(&mut self, step: usize, slot: i8) {
        if step < 64 {
            self.steps[step] = slot.clamp(-1, SLOT_COUNT as i8 - 1);
        }
    }
}

/// Bank of 8 pattern slots + one song sequence.
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct PatternBank {
    pub slots: [PatternSlot; SLOT_COUNT],
    pub song: SongSequence,
}

impl PatternSlot {
    /// Capture the current state from the plugin into this slot.
    pub fn capture(
        &mut self,
        pattern: &crate::sequencer::pattern::SharedPattern,
        plock_state: &crate::plock::PlockState,
        seq_plock_state: &crate::plock::SequencerPlockState,
        pattern_length: u8,
    ) {
        use crate::plock::{FIELD_COUNT, STEP_COUNT};
        use crate::sequencer::pattern::INSTRUMENT_COUNT;
        use std::sync::atomic::Ordering;

        self.step_masks = pattern.step_masks();

        // Serialize plock state — clear pre-allocated buffer (no new allocation).
        self.plock_bytes.clear();
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                for field in 0..FIELD_COUNT {
                    self.plock_bytes.extend_from_slice(
                        &plock_state.values.get(inst, step, field).to_le_bytes(),
                    );
                }
            }
        }
        for inst in 0..INSTRUMENT_COUNT {
            self.plock_bytes.extend_from_slice(
                &plock_state.masks.masks[inst]
                    .load(Ordering::Relaxed)
                    .to_le_bytes(),
            );
        }
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                self.plock_bytes
                    .extend_from_slice(&plock_state.field_masks.get(inst, step).to_le_bytes());
            }
        }

        // Serialize seq plock state — clear pre-allocated buffer (no new allocation).
        self.seq_plock_bytes.clear();
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                self.seq_plock_bytes.extend_from_slice(
                    &f32::from_bits(
                        seq_plock_state.probabilities[inst][step].load(Ordering::Relaxed),
                    )
                    .to_le_bytes(),
                );
                self.seq_plock_bytes.extend_from_slice(
                    &f32::from_bits(seq_plock_state.stutters[inst][step].load(Ordering::Relaxed))
                        .to_le_bytes(),
                );
                self.seq_plock_bytes.extend_from_slice(
                    &seq_plock_state.conditions[inst][step]
                        .load(Ordering::Relaxed)
                        .to_le_bytes(),
                );
                self.seq_plock_bytes.extend_from_slice(
                    &f32::from_bits(
                        seq_plock_state.microtimings[inst][step].load(Ordering::Relaxed),
                    )
                    .to_le_bytes(),
                );
            }
        }
        for inst in 0..INSTRUMENT_COUNT {
            self.seq_plock_bytes.extend_from_slice(
                &seq_plock_state.masks[inst]
                    .load(Ordering::Relaxed)
                    .to_le_bytes(),
            );
        }

        self.pattern_length = pattern_length.clamp(1, 64);
        self.occupied = true;
    }

    /// Copy slot data into temporary buffers so the caller can release the
    /// bank lock before doing the (expensive) restore. Returns the pattern
    /// length if the slot is occupied, otherwise None.
    pub fn copy_data_for_restore(
        &self,
        step_masks_out: &mut [u16; STEP_COUNT],
        plock_bytes_out: &mut [u8],
        seq_plock_bytes_out: &mut [u8],
    ) -> Option<u8> {
        if !self.occupied {
            return None;
        }
        step_masks_out.copy_from_slice(&self.step_masks);

        let plock_len = self.plock_bytes.len().min(plock_bytes_out.len());
        let seq_plock_len = self.seq_plock_bytes.len().min(seq_plock_bytes_out.len());
        plock_bytes_out[..plock_len].copy_from_slice(&self.plock_bytes[..plock_len]);
        seq_plock_bytes_out[..seq_plock_len]
            .copy_from_slice(&self.seq_plock_bytes[..seq_plock_len]);

        Some(self.pattern_length)
    }

    /// Restore this slot's data into the plugin state.
    pub fn restore(
        &self,
        pattern: &crate::sequencer::pattern::SharedPattern,
        plock_state: &crate::plock::PlockState,
        seq_plock_state: &crate::plock::SequencerPlockState,
    ) -> Option<u8> {
        use crate::plock::{FIELD_COUNT, STEP_COUNT};
        use crate::sequencer::pattern::INSTRUMENT_COUNT;
        use std::sync::atomic::Ordering;

        if !self.occupied {
            return None;
        }

        pattern.load_step_masks(&self.step_masks);

        // Detect field count from data size to support legacy slots (FIELD_COUNT=18)
        // and current slots (FIELD_COUNT=46).
        let legacy_field_count = 18usize;
        let current_expected = INSTRUMENT_COUNT * STEP_COUNT * FIELD_COUNT * 4
            + INSTRUMENT_COUNT * 8
            + INSTRUMENT_COUNT * STEP_COUNT * 8;
        let legacy_expected = INSTRUMENT_COUNT * STEP_COUNT * legacy_field_count * 4
            + INSTRUMENT_COUNT * 8
            + INSTRUMENT_COUNT * STEP_COUNT * 8;

        let field_count = if self.plock_bytes.len() >= current_expected {
            FIELD_COUNT
        } else if self.plock_bytes.len() >= legacy_expected {
            legacy_field_count
        } else {
            0
        };

        if field_count > 0 {
            let mut offset = 0usize;
            for inst in 0..INSTRUMENT_COUNT {
                for step in 0..STEP_COUNT {
                    for field in 0..field_count {
                        let val = f32::from_le_bytes([
                            self.plock_bytes[offset],
                            self.plock_bytes[offset + 1],
                            self.plock_bytes[offset + 2],
                            self.plock_bytes[offset + 3],
                        ]);
                        plock_state.values.set(inst, step, field, val);
                        offset += 4;
                    }
                }
            }
            for inst in 0..INSTRUMENT_COUNT {
                let mask = u64::from_le_bytes([
                    self.plock_bytes[offset],
                    self.plock_bytes[offset + 1],
                    self.plock_bytes[offset + 2],
                    self.plock_bytes[offset + 3],
                    self.plock_bytes[offset + 4],
                    self.plock_bytes[offset + 5],
                    self.plock_bytes[offset + 6],
                    self.plock_bytes[offset + 7],
                ]);
                offset += 8;
                plock_state.masks.masks[inst].store(mask, Ordering::Release);
            }
            for inst in 0..INSTRUMENT_COUNT {
                for step in 0..STEP_COUNT {
                    let mask = u64::from_le_bytes([
                        self.plock_bytes[offset],
                        self.plock_bytes[offset + 1],
                        self.plock_bytes[offset + 2],
                        self.plock_bytes[offset + 3],
                        self.plock_bytes[offset + 4],
                        self.plock_bytes[offset + 5],
                        self.plock_bytes[offset + 6],
                        self.plock_bytes[offset + 7],
                    ]);
                    offset += 8;
                    plock_state.field_masks.set(inst, step, mask as usize);
                }
            }
        }

        // Deserialize seq plock state
        let cell_count = INSTRUMENT_COUNT * STEP_COUNT;
        let expected_seq_size = cell_count * 4 * 4 + INSTRUMENT_COUNT * 8;
        if self.seq_plock_bytes.len() >= expected_seq_size {
            let mut offset = 0;
            for inst in 0..INSTRUMENT_COUNT {
                for step in 0..STEP_COUNT {
                    let prob = f32::from_le_bytes([
                        self.seq_plock_bytes[offset],
                        self.seq_plock_bytes[offset + 1],
                        self.seq_plock_bytes[offset + 2],
                        self.seq_plock_bytes[offset + 3],
                    ]);
                    offset += 4;
                    let stutter = f32::from_le_bytes([
                        self.seq_plock_bytes[offset],
                        self.seq_plock_bytes[offset + 1],
                        self.seq_plock_bytes[offset + 2],
                        self.seq_plock_bytes[offset + 3],
                    ]);
                    offset += 4;
                    let condition = u32::from_le_bytes([
                        self.seq_plock_bytes[offset],
                        self.seq_plock_bytes[offset + 1],
                        self.seq_plock_bytes[offset + 2],
                        self.seq_plock_bytes[offset + 3],
                    ]);
                    offset += 4;
                    let micro = f32::from_le_bytes([
                        self.seq_plock_bytes[offset],
                        self.seq_plock_bytes[offset + 1],
                        self.seq_plock_bytes[offset + 2],
                        self.seq_plock_bytes[offset + 3],
                    ]);
                    offset += 4;

                    seq_plock_state.probabilities[inst][step]
                        .store(prob.to_bits(), Ordering::Release);
                    seq_plock_state.stutters[inst][step]
                        .store(stutter.to_bits(), Ordering::Release);
                    seq_plock_state.conditions[inst][step].store(condition, Ordering::Release);
                    seq_plock_state.microtimings[inst][step]
                        .store(micro.to_bits(), Ordering::Release);
                }
            }
            for inst in 0..INSTRUMENT_COUNT {
                let mask = u64::from_le_bytes([
                    self.seq_plock_bytes[offset],
                    self.seq_plock_bytes[offset + 1],
                    self.seq_plock_bytes[offset + 2],
                    self.seq_plock_bytes[offset + 3],
                    self.seq_plock_bytes[offset + 4],
                    self.seq_plock_bytes[offset + 5],
                    self.seq_plock_bytes[offset + 6],
                    self.seq_plock_bytes[offset + 7],
                ]);
                offset += 8;
                seq_plock_state.masks[inst].store(mask, Ordering::Release);
            }
        }

        Some(self.pattern_length)
    }
}

/// Restore pattern / plock / seq-plock data from raw buffers.
/// This is the same logic as `PatternSlot::restore` but decoupled from the
/// slot so the bank lock can be released before the (expensive) restore runs.
pub fn restore_from_buffers(
    step_masks: &[u16; STEP_COUNT],
    plock_bytes: &[u8],
    seq_plock_bytes: &[u8],
    pattern: &crate::sequencer::pattern::SharedPattern,
    plock_state: &crate::plock::PlockState,
    seq_plock_state: &crate::plock::SequencerPlockState,
) {
    use crate::plock::{FIELD_COUNT, STEP_COUNT};
    use crate::sequencer::pattern::INSTRUMENT_COUNT;
    use std::sync::atomic::Ordering;

    pattern.load_step_masks(step_masks);

    // Detect field count from data size to support legacy slots (FIELD_COUNT=18)
    // and current slots (FIELD_COUNT=46).
    let legacy_field_count = 18usize;
    let current_expected = INSTRUMENT_COUNT * STEP_COUNT * FIELD_COUNT * 4
        + INSTRUMENT_COUNT * 8
        + INSTRUMENT_COUNT * STEP_COUNT * 8;
    let legacy_expected = INSTRUMENT_COUNT * STEP_COUNT * legacy_field_count * 4
        + INSTRUMENT_COUNT * 8
        + INSTRUMENT_COUNT * STEP_COUNT * 8;

    let (field_count, has_plock_data) = if plock_bytes.len() >= current_expected {
        (FIELD_COUNT, true)
    } else if plock_bytes.len() >= legacy_expected {
        (legacy_field_count, true)
    } else {
        (FIELD_COUNT, false)
    };

    if has_plock_data {
        let mut offset = 0usize;
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                for field in 0..field_count {
                    let val = f32::from_le_bytes([
                        plock_bytes[offset],
                        plock_bytes[offset + 1],
                        plock_bytes[offset + 2],
                        plock_bytes[offset + 3],
                    ]);
                    plock_state.values.set(inst, step, field, val);
                    offset += 4;
                }
            }
        }
        for inst in 0..INSTRUMENT_COUNT {
            let mask = u64::from_le_bytes([
                plock_bytes[offset],
                plock_bytes[offset + 1],
                plock_bytes[offset + 2],
                plock_bytes[offset + 3],
                plock_bytes[offset + 4],
                plock_bytes[offset + 5],
                plock_bytes[offset + 6],
                plock_bytes[offset + 7],
            ]);
            offset += 8;
            plock_state.masks.masks[inst].store(mask, Ordering::Release);
        }
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                let mask = u64::from_le_bytes([
                    plock_bytes[offset],
                    plock_bytes[offset + 1],
                    plock_bytes[offset + 2],
                    plock_bytes[offset + 3],
                    plock_bytes[offset + 4],
                    plock_bytes[offset + 5],
                    plock_bytes[offset + 6],
                    plock_bytes[offset + 7],
                ]);
                offset += 8;
                plock_state.field_masks.set(inst, step, mask as usize);
            }
        }
    }

    let cell_count = INSTRUMENT_COUNT * STEP_COUNT;
    let expected_seq_size = cell_count * 4 * 4 + INSTRUMENT_COUNT * 8;
    if seq_plock_bytes.len() >= expected_seq_size {
        let mut offset = 0;
        for inst in 0..INSTRUMENT_COUNT {
            for step in 0..STEP_COUNT {
                let prob = f32::from_le_bytes([
                    seq_plock_bytes[offset],
                    seq_plock_bytes[offset + 1],
                    seq_plock_bytes[offset + 2],
                    seq_plock_bytes[offset + 3],
                ]);
                offset += 4;
                let stutter = f32::from_le_bytes([
                    seq_plock_bytes[offset],
                    seq_plock_bytes[offset + 1],
                    seq_plock_bytes[offset + 2],
                    seq_plock_bytes[offset + 3],
                ]);
                offset += 4;
                let condition = u32::from_le_bytes([
                    seq_plock_bytes[offset],
                    seq_plock_bytes[offset + 1],
                    seq_plock_bytes[offset + 2],
                    seq_plock_bytes[offset + 3],
                ]);
                offset += 4;
                let micro = f32::from_le_bytes([
                    seq_plock_bytes[offset],
                    seq_plock_bytes[offset + 1],
                    seq_plock_bytes[offset + 2],
                    seq_plock_bytes[offset + 3],
                ]);
                offset += 4;

                seq_plock_state.probabilities[inst][step].store(prob.to_bits(), Ordering::Release);
                seq_plock_state.stutters[inst][step].store(stutter.to_bits(), Ordering::Release);
                seq_plock_state.conditions[inst][step].store(condition, Ordering::Release);
                seq_plock_state.microtimings[inst][step].store(micro.to_bits(), Ordering::Release);
            }
        }
        for inst in 0..INSTRUMENT_COUNT {
            let mask = u64::from_le_bytes([
                seq_plock_bytes[offset],
                seq_plock_bytes[offset + 1],
                seq_plock_bytes[offset + 2],
                seq_plock_bytes[offset + 3],
                seq_plock_bytes[offset + 4],
                seq_plock_bytes[offset + 5],
                seq_plock_bytes[offset + 6],
                seq_plock_bytes[offset + 7],
            ]);
            offset += 8;
            seq_plock_state.masks[inst].store(mask, Ordering::Release);
        }
    }
}

impl PatternBank {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Wrapper around PatternBank for nih-plug persistence.
/// Uses Arc so the UI and audio thread can share the same bank.
#[derive(Clone)]
pub struct PersistentPatternBank {
    pub bank: std::sync::Arc<std::sync::Mutex<PatternBank>>,
}

impl PersistentPatternBank {
    pub fn new() -> Self {
        Self {
            bank: std::sync::Arc::new(std::sync::Mutex::new(PatternBank::new())),
        }
    }
}

impl Default for PersistentPatternBank {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> nih_plug::params::persist::PersistentField<'a, Vec<u8>> for PersistentPatternBank {
    fn set(&self, new_value: Vec<u8>) {
        if let Ok(bank) = serde_json::from_slice::<PatternBank>(&new_value) {
            if let Ok(mut guard) = self.bank.lock() {
                *guard = bank;
            }
        }
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&Vec<u8>) -> R,
    {
        let bytes = if let Ok(guard) = self.bank.lock() {
            serde_json::to_vec(&*guard).unwrap_or_default()
        } else {
            Vec::new()
        };
        f(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plock::{PlockState, SequencerPlockState};
    use crate::sequencer::pattern::{Pattern, SharedPattern};
    use nih_plug::params::persist::PersistentField;

    #[test]
    fn pattern_slot_capture_restore_roundtrip() {
        let pattern = SharedPattern::new(&Pattern::rock_pattern());
        let plock = PlockState::new();
        let seq_plock = SequencerPlockState::new();

        // Set some plock data
        plock.values.set(0, 5, 0, 0.75);
        plock.masks.masks[0].store(1 << 5, std::sync::atomic::Ordering::Relaxed);
        plock.field_masks.set(0, 5, 1);

        // Set some seq plock data
        seq_plock.probabilities[0][5].store(0.5f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        seq_plock.masks[0].store(1 << 5, std::sync::atomic::Ordering::Relaxed);

        // Capture
        let mut slot = PatternSlot::default();
        slot.capture(&pattern, &plock, &seq_plock, 32);
        assert!(slot.occupied);
        assert_eq!(slot.pattern_length, 32);

        // Modify original state
        pattern.set_step_mask(0, 0);
        plock.values.set(0, 5, 0, 0.0);
        seq_plock.probabilities[0][5].store(1.0f32.to_bits(), std::sync::atomic::Ordering::Relaxed);

        // Restore
        let restored_len = slot.restore(&pattern, &plock, &seq_plock);
        assert_eq!(restored_len, Some(32));

        // Verify restored state
        assert_eq!(
            pattern.load_step_mask(0),
            Pattern::rock_pattern().step_masks()[0]
        );
        assert_eq!(plock.values.get(0, 5, 0), 0.75);
        assert_eq!(
            f32::from_bits(
                seq_plock.probabilities[0][5].load(std::sync::atomic::Ordering::Relaxed)
            ),
            0.5
        );
    }

    #[test]
    fn pattern_slot_preallocated_capacity_never_reallocates() {
        let slot = PatternSlot::default();
        assert!(slot.plock_bytes.capacity() >= MAX_PLOCK_BYTES);
        assert!(slot.seq_plock_bytes.capacity() >= MAX_SEQ_PLOCK_BYTES);
    }

    #[test]
    fn song_sequence_slot_at_and_set() {
        let mut song = SongSequence::new();
        song.length = 4;
        song.set_step(0, 2); // P3
        song.set_step(1, -1); // empty
        song.set_step(2, 7); // P8
        song.set_step(3, 0); // P1

        assert_eq!(song.slot_at(0), Some(2));
        assert_eq!(song.slot_at(1), None);
        assert_eq!(song.slot_at(2), Some(7));
        assert_eq!(song.slot_at(3), Some(0));
        assert_eq!(song.slot_at(4), None); // beyond length
    }

    #[test]
    fn pattern_bank_persistence_roundtrips_song() {
        let persistent = PersistentPatternBank::new();
        {
            let mut guard = persistent.bank.lock().unwrap();
            guard.song.length = 4;
            guard.song.set_step(0, 1);
            guard.song.set_step(1, 3);
            guard.song.loop_enabled = true;
        }

        let bytes = persistent.map(|b| b.clone());

        let restored = PersistentPatternBank::new();
        restored.set(bytes);

        let guard = restored.bank.lock().unwrap();
        assert_eq!(guard.song.length, 4);
        assert_eq!(guard.song.slot_at(0), Some(1));
        assert_eq!(guard.song.slot_at(1), Some(3));
        assert!(guard.song.loop_enabled);
    }

    #[test]
    fn pattern_bank_switch_clears_plocks() {
        // Reproduce the user bug: save pattern A with plocks to P1,
        // save pattern B with different plocks to P2, load P1 then P2
        // and verify plocks don't leak.
        let pattern_a = SharedPattern::new(&Pattern::rock_pattern());
        let pattern_b = SharedPattern::new(&Pattern::funk_pattern());
        let plock = PlockState::new();
        let seq_plock = SequencerPlockState::new();

        // Set plock on pattern A (instrument 0, step 5, volume = 0.75)
        plock.set_field(0, 5, 2, 0.75);
        // Set seq plock on pattern A (instrument 0, step 5, probability = 0.5)
        seq_plock.set_probability(0, 5, 0.5);

        // Save pattern A to slot 0
        let mut slot0 = PatternSlot::default();
        slot0.capture(&pattern_a, &plock, &seq_plock, 16);

        // Clear and set DIFFERENT plocks for pattern B
        // (instrument 1, step 10, volume = 0.9)
        plock.clear_all();
        seq_plock.clear_all();
        plock.set_field(1, 10, 2, 0.9);
        seq_plock.set_probability(1, 10, 0.25);

        // Save pattern B to slot 1
        let mut slot1 = PatternSlot::default();
        slot1.capture(&pattern_b, &plock, &seq_plock, 16);

        // Now simulate loading slot 0 (pattern A)
        // Use fresh shared pattern and plock state
        let target_pattern = SharedPattern::new(&Pattern::empty());
        let target_plock = PlockState::new();
        let target_seq_plock = SequencerPlockState::new();

        // Load slot 0
        slot0.restore(&target_pattern, &target_plock, &target_seq_plock);

        // Verify pattern A plocks are present
        assert!(target_plock.masks.is_active(0, 5));
        assert_eq!(target_plock.values.get(0, 5, 2), 0.75);
        assert!(target_seq_plock.is_active(0, 5));
        assert_eq!(
            f32::from_bits(
                target_seq_plock.probabilities[0][5].load(std::sync::atomic::Ordering::Relaxed)
            ),
            0.5
        );
        // Pattern B plocks should NOT be present
        assert!(!target_plock.masks.is_active(1, 10));

        // Now load slot 1 (pattern B) - this is where the bug happens
        // First clear, then restore (same as load_pattern_from_slot does)
        target_plock.clear_all();
        target_seq_plock.clear_all();
        slot1.restore(&target_pattern, &target_plock, &target_seq_plock);

        // Verify pattern B plocks are present
        assert!(target_plock.masks.is_active(1, 10));
        assert_eq!(target_plock.values.get(1, 10, 2), 0.9);
        assert!(target_seq_plock.is_active(1, 10));
        assert_eq!(
            f32::from_bits(
                target_seq_plock.probabilities[1][10].load(std::sync::atomic::Ordering::Relaxed)
            ),
            0.25
        );
        // Pattern A plocks should NOT be present anymore
        assert!(
            !target_plock.masks.is_active(0, 5),
            "Plock from pattern A leaked into pattern B!"
        );
        assert!(
            !target_seq_plock.is_active(0, 5),
            "Seq plock from pattern A leaked into pattern B!"
        );
    }
}
