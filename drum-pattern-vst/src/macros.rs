//! [242] MIDI CC macros: sixteen host-visible "Macro N" params (0..1) the DAW
//! drives (Control Link / automation), each assignable to one SOUND parameter
//! of an instantiated lane. Once per buffer the audio thread maps a changed
//! macro value into the lane's per-slot atomics — the exact path a UI drag
//! takes — so p-locks keep overriding per step and the voices pick the value
//! up through the usual settings-version poll.
//!
//! Scope (user decision, 2026-09-22): standards + specials only. Sequencer
//! p-locks (per-step values), structural params and the per-lane nih-plug
//! params (mute/solo/algo — the host-notification trap of [227]) are out.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::instrument_registry::{self, ParamWidget, StandardField};
use crate::sound_settings::{InstrumentSettingsState, SoundSettingsState};
use crate::track::{AtomicTrackLayout, MAX_TRACKS};

/// Macro knobs exposed to the host today. The STORE below is sized for 32 so
/// extending later is "add sixteen params" — not a persistence-format bump.
pub const MACRO_COUNT: usize = 16;
/// Store capacity (and the `macro-map-v1` blob length): room for 32.
pub const MACRO_SLOTS: usize = 32;

/// One macro assignment: a lane plus one of its sound parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroTarget {
    Std(usize, StandardField),
    Special(usize, usize),
}

impl MacroTarget {
    pub fn slot(self) -> usize {
        match self {
            MacroTarget::Std(slot, _) | MacroTarget::Special(slot, _) => slot,
        }
    }

    fn with_slot(self, slot: usize) -> Self {
        match self {
            MacroTarget::Std(_, field) => MacroTarget::Std(slot, field),
            MacroTarget::Special(_, idx) => MacroTarget::Special(slot, idx),
        }
    }
}

/// Packed entry: 0 = unassigned; else bits 0..=3 = slot+1, bit 4 = kind
/// (0 = standard, 1 = special), bits 5..=12 = field / special index.
pub fn pack_target(target: Option<MacroTarget>) -> u32 {
    match target {
        None => 0,
        Some(MacroTarget::Std(slot, field)) => {
            let idx = StandardField::ALL.iter().position(|f| *f == field).unwrap_or(0) as u32;
            (slot as u32 + 1) | (idx << 5)
        }
        Some(MacroTarget::Special(slot, idx)) => (slot as u32 + 1) | (1 << 4) | ((idx as u32) << 5),
    }
}

pub fn unpack_target(packed: u32) -> Option<MacroTarget> {
    let slot = (packed & 0xF).checked_sub(1)? as usize;
    if slot >= MAX_TRACKS {
        return None;
    }
    let idx = ((packed >> 5) & 0xFF) as usize;
    if packed & (1 << 4) != 0 {
        if idx >= 32 {
            return None;
        }
        Some(MacroTarget::Special(slot, idx))
    } else {
        StandardField::ALL.get(idx).map(|f| MacroTarget::Std(slot, *f))
    }
}

/// Map a 0..1 macro value onto a parameter's range, honouring log sliders
/// (Freq, FilterFreq, …) so the CC travels the way the slider does.
pub fn scale_value(t: f32, min: f32, max: f32, logarithmic: bool) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if logarithmic && min > 0.0 && max > min {
        min * (max / min).powf(t)
    } else {
        min + t * (max - min)
    }
}

/// (min, max, logarithmic) of a target on a given voice, from the registry.
/// None when the lane's kind doesn't declare that parameter (the assignment
/// is stale — the UI prunes those once per frame).
pub fn target_range(voice_idx: usize, target: MacroTarget) -> Option<(f32, f32, bool)> {
    let def = &instrument_registry::INSTRUMENTS[voice_idx];
    match target {
        MacroTarget::Std(_, field) => {
            let d = def.standard_params.iter().find(|d| d.field == field)?;
            match d.widget {
                ParamWidget::Slider {
                    min,
                    max,
                    logarithmic,
                    ..
                } => Some((min, max, logarithmic)),
                ParamWidget::Checkbox => Some((0.0, 1.0, false)),
            }
        }
        MacroTarget::Special(_, idx) => def
            .special_params
            .iter()
            .find(|d| d.special_index == idx)
            .map(|d| (d.min, d.max, false)),
    }
}

/// The thirty-two assignment slots, atomic for UI-write / audio-read.
pub struct MacroMap {
    entries: [AtomicU32; MACRO_SLOTS],
}

impl MacroMap {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            entries: std::array::from_fn(|_| AtomicU32::new(0)),
        })
    }

    pub fn get(&self, macro_idx: usize) -> Option<MacroTarget> {
        self.entries
            .get(macro_idx)
            .and_then(|e| unpack_target(e.load(Ordering::Relaxed)))
    }

    pub fn set(&self, macro_idx: usize, target: Option<MacroTarget>) {
        if let Some(e) = self.entries.get(macro_idx) {
            e.store(pack_target(target), Ordering::Relaxed);
        }
    }

    /// Every assignment pointing at a lane (for the UI list).
    pub fn targets_for_slot(&self, slot: usize) -> Vec<(usize, MacroTarget)> {
        (0..MACRO_SLOTS)
            .filter_map(|i| self.get(i).map(|t| (i, t)))
            .filter(|(_, t)| t.slot() == slot)
            .collect()
    }

    /// Lane reorder: `order[new] = old`, like every other per-slot store. The
    /// slot is packed INSIDE each entry, so entries are rewritten to their
    /// lane's new number ([228] pitfall: a store left behind then pruned).
    pub fn reorder(&self, order: &[usize]) {
        for entry in self.entries.iter() {
            let new = match unpack_target(entry.load(Ordering::Relaxed)) {
                None => 0,
                Some(t) => {
                    let new_slot = order
                        .iter()
                        .position(|&old| old == t.slot())
                        .unwrap_or_else(|| t.slot());
                    pack_target(Some(t.with_slot(new_slot)))
                }
            };
            entry.store(new, Ordering::Relaxed);
        }
    }

    /// Drop assignments whose lane no longer offers the target: lane removed,
    /// kind changed (a special index means something else on the new kind —
    /// the [228] texture rule), retired voice. Standards and specials alike:
    /// the check is "declared by the current kind", so a standard the new
    /// kind doesn't have goes too. Returns whether anything was pruned.
    pub fn prune_invalid_targets(&self, layout: &AtomicTrackLayout) -> bool {
        let mut pruned = false;
        for i in 0..MACRO_SLOTS {
            let Some(target) = self.get(i) else { continue };
            let valid = layout
                .kind_for_slot(target.slot())
                .map(|kind| {
                    let def = kind.instrument_def();
                    match target {
                        MacroTarget::Std(_, field) => {
                            def.standard_params.iter().any(|d| d.field == field)
                        }
                        MacroTarget::Special(_, idx) => {
                            def.special_params.iter().any(|d| d.special_index == idx)
                        }
                    }
                })
                .unwrap_or(false);
            if !valid {
                self.set(i, None);
                pruned = true;
            }
        }
        pruned
    }
}

/// Push one assigned macro value into a lane's atomics. Pure on the params
/// side (the value arrives already read) so the headless tests drive it
/// directly. Returns whether the lane was written.
pub fn apply_one(
    value: f32,
    target: MacroTarget,
    voice_idx: usize,
    inst: &InstrumentSettingsState,
) -> bool {
    let Some((min, max, log)) = target_range(voice_idx, target) else {
        return false;
    };
    let scaled = scale_value(value, min, max, log);
    match target {
        MacroTarget::Std(_, field) => inst.set_standard(field, scaled),
        MacroTarget::Special(_, idx) => inst.set_special(idx, scaled),
    }
    true
}

/// Once per buffer, BEFORE the settings poll: apply every assigned macro
/// whose value moved since the last buffer, then bump the settings version
/// once so the voices pick the writes up through the usual path. RT-safe:
/// atomic loads/stores only.
pub fn apply_macros(
    macro_values: [f32; MACRO_COUNT],
    map: &MacroMap,
    sound_settings: &SoundSettingsState,
    slot_voices: &[Option<usize>; MAX_TRACKS],
) {
    let mut touched = false;
    for (i, value) in macro_values.iter().copied().enumerate() {
        let Some(target) = map.get(i) else { continue };
        let Some(voice_idx) = slot_voices[target.slot()] else {
            continue;
        };
        if apply_one(value, target, voice_idx, &sound_settings.instruments[target.slot()]) {
            touched = true;
        }
    }
    if touched {
        sound_settings.bump_version();
    }
}

/// Persistence wrapper — `macro-map-v1` blob: MACRO_SLOTS × u32 LE. The
/// length IS the version, like every blob here; extending to 32 macros keeps
/// the length, so no v2.
pub struct PersistentMacroMap {
    pub state: Arc<MacroMap>,
}

impl PersistentMacroMap {
    pub fn new() -> Self {
        Self {
            state: MacroMap::new(),
        }
    }
}

impl<'a> nih_plug::params::persist::PersistentField<'a, Vec<u8>> for PersistentMacroMap {
    fn set(&self, new_value: Vec<u8>) {
        if new_value.len() != MACRO_SLOTS * 4 {
            return;
        }
        for (i, chunk) in new_value.chunks_exact(4).enumerate() {
            let packed = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            // Validate on load: a corrupt or future entry drops to unassigned
            // rather than pointing at a nonsense slot.
            let packed = if unpack_target(packed).is_some() {
                packed
            } else {
                0
            };
            self.state.entries[i].store(packed, Ordering::Relaxed);
        }
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&Vec<u8>) -> R,
    {
        let mut result = Vec::with_capacity(MACRO_SLOTS * 4);
        for entry in self.state.entries.iter() {
            result.extend_from_slice(&entry.load(Ordering::Relaxed).to_le_bytes());
        }
        f(&result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nih_plug::params::persist::PersistentField;

    #[test]
    fn pack_unpack_roundtrip() {
        assert_eq!(unpack_target(0), None);
        for t in [
            MacroTarget::Std(0, StandardField::Freq),
            MacroTarget::Std(13, StandardField::Stereo),
            MacroTarget::Special(4, 0),
            MacroTarget::Special(13, 31),
        ] {
            assert_eq!(unpack_target(pack_target(Some(t))), Some(t));
        }
        // Out-of-range slot/index loads as unassigned, never panics.
        assert_eq!(unpack_target(0xF), None);
        assert_eq!(unpack_target((1) | (1 << 4) | (40 << 5)), None);
    }

    #[test]
    fn scale_value_linear_log_and_clamped() {
        assert_eq!(scale_value(0.5, 0.0, 2.0, false), 1.0);
        assert_eq!(scale_value(2.0, 0.0, 2.0, false), 2.0);
        // Log: mid-travel is the geometric mean, like the Freq slider.
        let mid = scale_value(0.5, 20.0, 20000.0, true);
        assert!((mid - 632.45).abs() < 1.0, "got {mid}");
        // Degenerate log range falls back to linear rather than NaN.
        assert_eq!(scale_value(0.5, 0.0, 1.0, true), 0.5);
    }

    #[test]
    fn target_range_reads_the_registry() {
        // Kick (voice 0) Decay is a linear slider; Rift's Offset special exists.
        let (min, max, log) = target_range(0, MacroTarget::Std(0, StandardField::Decay)).unwrap();
        assert!(min < max);
        assert!(!log);
        let kick = instrument_registry::INSTRUMENTS[0]
            .standard_params
            .iter()
            .find(|d| d.field == StandardField::Decay)
            .unwrap();
        let ParamWidget::Slider { min: dmin, max: dmax, .. } = kick.widget else {
            panic!("Decay should be a slider");
        };
        assert_eq!((min, max), (dmin, dmax));
        // A special the Kick doesn't declare has no range.
        assert!(target_range(0, MacroTarget::Special(0, 31)).is_none());
    }

    #[test]
    fn apply_macros_writes_the_lane_atomics() {
        let layout = crate::track::TrackLayoutState::modular_default_layout();
        let settings = SoundSettingsState::new(&layout);
        let map = MacroMap::new();
        map.set(0, Some(MacroTarget::Std(0, StandardField::Decay)));
        map.set(1, Some(MacroTarget::Special(1, 99))); // not declared -> skipped
        let mut values = [0.0; MACRO_COUNT];
        values[0] = 1.0;
        let slot_voices: [Option<usize>; MAX_TRACKS] = std::array::from_fn(|i| {
            if layout.slots[i].active {
                Some(layout.slots[i].kind.drum_voice_index())
            } else {
                None
            }
        });
        let before = settings.version.load(Ordering::Acquire);
        apply_macros(values, &map, &settings, &slot_voices);
        let (_, max, _) = target_range(0, MacroTarget::Std(0, StandardField::Decay)).unwrap();
        assert_eq!(
            settings.instruments[0].decay.load(Ordering::Relaxed),
            max.to_bits()
        );
        assert!(settings.version.load(Ordering::Acquire) > before);
    }

    #[test]
    fn reorder_renumbers_the_packed_slot() {
        let map = MacroMap::new();
        map.set(3, Some(MacroTarget::Std(1, StandardField::Freq)));
        // Lane 1 moves to lane 3 (order[new] = old), lanes 2/3 shift up.
        let mut order: Vec<usize> = (0..MAX_TRACKS).collect();
        order[1] = 2;
        order[2] = 3;
        order[3] = 1;
        map.reorder(&order);
        assert_eq!(map.get(3), Some(MacroTarget::Std(3, StandardField::Freq)));
    }

    #[test]
    fn prune_drops_what_the_kind_no_longer_offers() {
        let mut layout = crate::track::TrackLayoutState::modular_default_layout();
        let map = MacroMap::new();
        // Slot 0 hosts a Kick: Decay exists, special 31 does not.
        map.set(0, Some(MacroTarget::Std(0, StandardField::Decay)));
        map.set(1, Some(MacroTarget::Special(0, 31)));
        // Slot 9 is inactive.
        map.set(2, Some(MacroTarget::Std(9, StandardField::Freq)));
        assert!(map.prune_invalid_targets(&AtomicTrackLayout::from_state(&layout)));
        assert_eq!(map.get(0), Some(MacroTarget::Std(0, StandardField::Decay)));
        assert_eq!(map.get(1), None);
        assert_eq!(map.get(2), None);
        // Nothing left to prune.
        layout.slots[0].name = String::new();
        assert!(!map.prune_invalid_targets(&AtomicTrackLayout::from_state(&layout)));
    }

    #[test]
    fn persistence_blob_roundtrip_and_length_is_the_version() {
        let map = PersistentMacroMap::new();
        map.state.set(5, Some(MacroTarget::Special(2, 7)));
        let blob = map
            .map(|b| b.clone());
        assert_eq!(blob.len(), MACRO_SLOTS * 4);
        let restored = PersistentMacroMap::new();
        restored.set(blob);
        assert_eq!(restored.state.get(5), Some(MacroTarget::Special(2, 7)));
        // Wrong length (older/newer format): the blob is IGNORED and the
        // current assignments stay — same contract as the plock blobs.
        restored.set(vec![0u8; 12]);
        assert_eq!(restored.state.get(5), Some(MacroTarget::Special(2, 7)));
    }
}
