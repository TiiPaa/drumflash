//! Modular track model.
//!
//! The plugin exposes a fixed internal pool of `MAX_TRACKS` slots. Only the
//! active slots are visible in the UI. Each active slot holds an instrument
//! kind, its own sound settings, routing, MIDI note and pattern data.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

pub const MAX_TRACKS: usize = 14;

/// Functional instrument kind. 11 fixed types; the three legacy Tom voices
/// collapse into a single `Tom` kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TrackInstrumentKind {
    Kick = 0,
    Snare = 1,
    HiHat = 2,
    OpenHiHat = 3,
    Tom = 4,
    Clap = 5,
    Ride = 6,
    Cymbal = 7,
    Snare606 = 8,
    BassDrum808 = 9,
    Perc1 = 10,
}

impl TrackInstrumentKind {
    pub const COUNT: usize = 11;

    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Kick),
            1 => Some(Self::Snare),
            2 => Some(Self::HiHat),
            3 => Some(Self::OpenHiHat),
            4 => Some(Self::Tom),
            5 => Some(Self::Clap),
            6 => Some(Self::Ride),
            7 => Some(Self::Cymbal),
            8 => Some(Self::Snare606),
            9 => Some(Self::BassDrum808),
            10 => Some(Self::Perc1),
            _ => None,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn default_label(self) -> &'static str {
        match self {
            TrackInstrumentKind::Kick => "BD",
            TrackInstrumentKind::Snare => "SD",
            TrackInstrumentKind::HiHat => "HH",
            TrackInstrumentKind::OpenHiHat => "OH",
            TrackInstrumentKind::Tom => "TM",
            TrackInstrumentKind::Clap => "CL",
            TrackInstrumentKind::Ride => "RD",
            TrackInstrumentKind::Cymbal => "CY",
            TrackInstrumentKind::Snare606 => "S6",
            TrackInstrumentKind::BassDrum808 => "B8",
            TrackInstrumentKind::Perc1 => "P1",
        }
    }

    pub fn default_name(self) -> &'static str {
        match self {
            TrackInstrumentKind::Kick => "Kick",
            TrackInstrumentKind::Snare => "Snare",
            TrackInstrumentKind::HiHat => "Hi-Hat",
            TrackInstrumentKind::OpenHiHat => "Open Hi-Hat",
            TrackInstrumentKind::Tom => "Tom",
            TrackInstrumentKind::Clap => "Clap",
            TrackInstrumentKind::Ride => "Ride",
            TrackInstrumentKind::Cymbal => "Cymbal",
            TrackInstrumentKind::Snare606 => "Snare 606",
            TrackInstrumentKind::BassDrum808 => "808 Kick",
            TrackInstrumentKind::Perc1 => "Perc1",
        }
    }

    /// Default GM drum note for this kind.
    pub fn default_midi_note(self) -> u8 {
        match self {
            TrackInstrumentKind::Kick => 36,
            TrackInstrumentKind::Snare => 38,
            TrackInstrumentKind::HiHat => 42,
            TrackInstrumentKind::OpenHiHat => 46,
            TrackInstrumentKind::Tom => 50,
            TrackInstrumentKind::Clap => 39,
            TrackInstrumentKind::Ride => 51,
            TrackInstrumentKind::Cymbal => 49,
            TrackInstrumentKind::Snare606 => 40,
            TrackInstrumentKind::BassDrum808 => 35,
            TrackInstrumentKind::Perc1 => 37,
        }
    }

    /// Map to the legacy `DrumVoice` index used by the synthesis layer.
    /// Tom always maps to the first tom variant because `DrumVoice` keeps
    /// three tom slots for backward compatibility while the track model has
    /// a single `Tom` kind.
    pub fn drum_voice_index(self) -> usize {
        match self {
            TrackInstrumentKind::Kick => 0,
            TrackInstrumentKind::Snare => 1,
            TrackInstrumentKind::HiHat => 2,
            TrackInstrumentKind::OpenHiHat => 3,
            TrackInstrumentKind::Tom => 4,
            TrackInstrumentKind::Clap => 7,
            TrackInstrumentKind::Ride => 8,
            TrackInstrumentKind::Cymbal => 9,
            TrackInstrumentKind::Snare606 => 10,
            TrackInstrumentKind::BassDrum808 => 11,
            TrackInstrumentKind::Perc1 => 12,
        }
    }

    /// Build from a legacy 13-voice `DrumVoice` index.
    pub fn from_drum_voice_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Kick),
            1 => Some(Self::Snare),
            2 => Some(Self::HiHat),
            3 => Some(Self::OpenHiHat),
            4 | 5 | 6 => Some(Self::Tom),
            7 => Some(Self::Clap),
            8 => Some(Self::Ride),
            9 => Some(Self::Cymbal),
            10 => Some(Self::Snare606),
            11 => Some(Self::BassDrum808),
            12 => Some(Self::Perc1),
            _ => None,
        }
    }

    pub fn algo_count(self) -> usize {
        crate::instrument_registry::INSTRUMENTS[self.drum_voice_index()].algo_count
    }

    pub fn instrument_def(self) -> &'static crate::instrument_registry::InstrumentDef {
        &crate::instrument_registry::INSTRUMENTS[self.drum_voice_index()]
    }
}

/// Selected audio destination for a track.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TrackAudioOut {
    Main,
    Out(u8), // 1..=MAX_TRACKS
}

impl TrackAudioOut {
    pub fn label(self) -> String {
        match self {
            TrackAudioOut::Main => "No Aux".to_string(),
            TrackAudioOut::Out(n) => format!("Out {}", n),
        }
    }

    pub fn from_index(index: u8) -> Self {
        if index == 0 {
            TrackAudioOut::Main
        } else {
            TrackAudioOut::Out(index.min(MAX_TRACKS as u8))
        }
    }

    pub fn index(self) -> u8 {
        match self {
            TrackAudioOut::Main => 0,
            TrackAudioOut::Out(n) => n,
        }
    }
}

/// Per-track audio routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrackRouting {
    pub main_on: bool,
    pub out_select: TrackAudioOut,
}

impl Default for TrackRouting {
    fn default() -> Self {
        Self {
            main_on: true,
            out_select: TrackAudioOut::Main,
        }
    }
}

/// One internal track slot.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrackSlot {
    pub active: bool,
    pub name: String,
    pub kind: TrackInstrumentKind,
    pub routing: TrackRouting,
    pub midi_note: u8,
}

impl TrackSlot {
    pub fn inactive() -> Self {
        Self {
            active: false,
            name: String::new(),
            kind: TrackInstrumentKind::Kick,
            routing: TrackRouting::default(),
            midi_note: TrackInstrumentKind::Kick.default_midi_note(),
        }
    }

    pub fn active_with_kind(kind: TrackInstrumentKind) -> Self {
        Self {
            active: true,
            name: kind.default_name().to_string(),
            kind,
            routing: TrackRouting::default(),
            midi_note: kind.default_midi_note(),
        }
    }
}

/// Full track layout persisted in the DAW state.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TrackLayoutState {
    #[serde(with = "serde_arrays")]
    pub slots: [TrackSlot; MAX_TRACKS],
    pub global_midi_channel: u8,
    pub global_base_note: u8,
}

impl TrackLayoutState {
    pub fn default_layout() -> Self {
        // Product decision 2026-07-04: new instances start with the modular
        // 4-lane template (BD/SD/HH/Tom). Legacy 13-voice sessions keep their
        // saved `track-layout-v1`; sessions saved BEFORE that field existed
        // will open with 4 lanes.
        Self::modular_default_layout()
    }

    pub fn modular_default_layout() -> Self {
        let mut slots: [TrackSlot; MAX_TRACKS] = std::array::from_fn(|_| TrackSlot::inactive());
        // Default template: BD / SD / HH / Tom
        slots[0] = TrackSlot::active_with_kind(TrackInstrumentKind::Kick);
        slots[1] = TrackSlot::active_with_kind(TrackInstrumentKind::Snare);
        slots[2] = TrackSlot::active_with_kind(TrackInstrumentKind::HiHat);
        slots[3] = TrackSlot::active_with_kind(TrackInstrumentKind::Tom);
        Self {
            slots,
            global_midi_channel: 10,
            global_base_note: 36,
        }
    }

    pub fn empty_layout() -> Self {
        Self {
            slots: std::array::from_fn(|_| TrackSlot::inactive()),
            global_midi_channel: 10,
            global_base_note: 36,
        }
    }

    pub fn preset_12_layout() -> Self {
        let mut slots: [TrackSlot; MAX_TRACKS] = std::array::from_fn(|_| TrackSlot::inactive());
        let kinds = [
            TrackInstrumentKind::Kick,
            TrackInstrumentKind::Snare,
            TrackInstrumentKind::HiHat,
            TrackInstrumentKind::OpenHiHat,
            TrackInstrumentKind::Tom,
            TrackInstrumentKind::Tom,
            TrackInstrumentKind::Tom,
            TrackInstrumentKind::Clap,
            TrackInstrumentKind::Ride,
            TrackInstrumentKind::Cymbal,
            TrackInstrumentKind::Snare606,
            TrackInstrumentKind::BassDrum808,
        ];
        for (slot, kind) in slots.iter_mut().zip(kinds) {
            *slot = TrackSlot::active_with_kind(kind);
        }
        Self {
            slots,
            global_midi_channel: 10,
            global_base_note: 36,
        }
    }

    /// Migrate a legacy 13-voice session into 14 slots.
    pub fn from_legacy_13() -> Self {
        let mut slots: [TrackSlot; MAX_TRACKS] = std::array::from_fn(|_| TrackSlot::inactive());
        let legacy_kinds = [
            TrackInstrumentKind::Kick,
            TrackInstrumentKind::Snare,
            TrackInstrumentKind::HiHat,
            TrackInstrumentKind::OpenHiHat,
            TrackInstrumentKind::Tom,
            TrackInstrumentKind::Tom,
            TrackInstrumentKind::Tom,
            TrackInstrumentKind::Clap,
            TrackInstrumentKind::Ride,
            TrackInstrumentKind::Cymbal,
            TrackInstrumentKind::Snare606,
            TrackInstrumentKind::BassDrum808,
            TrackInstrumentKind::Perc1,
        ];
        for (i, kind) in legacy_kinds.iter().enumerate() {
            slots[i] = TrackSlot {
                active: true,
                name: format!("{} {}", kind.default_name(), occurrence_label(i, *kind)),
                kind: *kind,
                routing: TrackRouting::default(),
                midi_note: crate::instrument_registry::INSTRUMENTS[i].midi_note,
            };
        }
        Self {
            slots,
            global_midi_channel: 10,
            global_base_note: 36,
        }
    }

    pub fn active_slot_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.active)
            .map(|(i, _)| i)
    }

    pub fn active_count(&self) -> usize {
        self.slots.iter().filter(|s| s.active).count()
    }

    pub fn move_slot(&mut self, from: usize, to: usize) {
        if from >= MAX_TRACKS || to >= MAX_TRACKS || from == to {
            return;
        }

        let moved = self.slots[from].clone();
        if from < to {
            for idx in from..to {
                self.slots[idx] = self.slots[idx + 1].clone();
            }
        } else {
            for idx in (to + 1..=from).rev() {
                self.slots[idx] = self.slots[idx - 1].clone();
            }
        }
        self.slots[to] = moved;
    }

    pub fn first_inactive_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| !s.active)
    }

    pub fn assign_slot_output_exclusive(&mut self, slot: usize, output: TrackAudioOut) {
        if slot >= MAX_TRACKS {
            return;
        }

        if let TrackAudioOut::Out(out_number) = output {
            for (other_idx, other_slot) in self.slots.iter_mut().enumerate() {
                if other_idx != slot
                    && other_slot.routing.out_select == TrackAudioOut::Out(out_number)
                {
                    other_slot.routing.out_select = TrackAudioOut::Main;
                }
            }
        }

        self.slots[slot].routing.out_select = output;
    }
}

fn occurrence_label(slot: usize, kind: TrackInstrumentKind) -> String {
    // Produce "Tom 1", "Tom 2", "Tom 3" for duplicate toms, empty otherwise.
    if kind == TrackInstrumentKind::Tom {
        let tom_count_before = (0..slot)
            .filter(|&i| {
                i < MAX_TRACKS
                    && TrackInstrumentKind::from_drum_voice_index(i)
                        == Some(TrackInstrumentKind::Tom)
            })
            .count();
        return format!("{}", tom_count_before + 1);
    }
    String::new()
}

/// Atomic, lock-free view of the track layout used by the audio thread.
pub struct AtomicTrackLayout {
    pub slot_kinds: [AtomicU8; MAX_TRACKS],
    pub slot_routing: [AtomicU8; MAX_TRACKS],
    pub slot_midi_notes: [AtomicU8; MAX_TRACKS],
    pub global_channel: AtomicU8,
    pub global_base_note: AtomicU8,
    pub version: std::sync::atomic::AtomicU64,
}

impl AtomicTrackLayout {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            slot_kinds: std::array::from_fn(|_| AtomicU8::new(0xFF)),
            slot_routing: std::array::from_fn(|_| AtomicU8::new(0)),
            slot_midi_notes: std::array::from_fn(|_| AtomicU8::new(0)),
            global_channel: AtomicU8::new(10),
            global_base_note: AtomicU8::new(36),
            version: std::sync::atomic::AtomicU64::new(0),
        })
    }

    pub fn from_state(state: &TrackLayoutState) -> Arc<Self> {
        let atomic = Self::new();
        atomic.update_from_state(state);
        atomic
    }

    pub fn update_from_state(&self, state: &TrackLayoutState) {
        for (i, slot) in state.slots.iter().enumerate() {
            let kind_byte = if slot.active {
                slot.kind.index() as u8
            } else {
                0xFF
            };
            self.slot_kinds[i].store(kind_byte, Ordering::Relaxed);
            let routing_byte = routing_byte(&slot.routing);
            self.slot_routing[i].store(routing_byte, Ordering::Relaxed);
            self.slot_midi_notes[i].store(slot.midi_note, Ordering::Relaxed);
        }
        self.global_channel
            .store(state.global_midi_channel.clamp(1, 16), Ordering::Relaxed);
        self.global_base_note
            .store(state.global_base_note, Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Release);
    }

    pub fn kind_for_slot(&self, slot: usize) -> Option<TrackInstrumentKind> {
        if slot >= MAX_TRACKS {
            return None;
        }
        let byte = self.slot_kinds[slot].load(Ordering::Relaxed);
        if byte == 0xFF {
            None
        } else {
            TrackInstrumentKind::from_index(byte as usize)
        }
    }

    pub fn is_active(&self, slot: usize) -> bool {
        slot < MAX_TRACKS && self.slot_kinds[slot].load(Ordering::Relaxed) != 0xFF
    }

    pub fn routing_for_slot(&self, slot: usize) -> TrackRouting {
        if slot >= MAX_TRACKS {
            return TrackRouting::default();
        }
        decode_routing(self.slot_routing[slot].load(Ordering::Relaxed))
    }

    pub fn midi_note_for_slot(&self, slot: usize) -> u8 {
        if slot >= MAX_TRACKS {
            return 0;
        }
        self.slot_midi_notes[slot].load(Ordering::Relaxed)
    }

    pub fn global_midi_channel(&self) -> u8 {
        self.global_channel.load(Ordering::Relaxed)
    }

    pub fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Release);
    }
}

impl Default for AtomicTrackLayout {
    fn default() -> Self {
        // Only used when we need a standalone value; the plugin always uses Arc.
        Self {
            slot_kinds: std::array::from_fn(|_| AtomicU8::new(0xFF)),
            slot_routing: std::array::from_fn(|_| AtomicU8::new(0)),
            slot_midi_notes: std::array::from_fn(|_| AtomicU8::new(0)),
            global_channel: AtomicU8::new(10),
            global_base_note: AtomicU8::new(36),
            version: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

fn routing_byte(routing: &TrackRouting) -> u8 {
    let main_bit = if routing.main_on { 0x80 } else { 0 };
    let out_index = routing.out_select.index();
    main_bit | (out_index & 0x7F)
}

fn decode_routing(byte: u8) -> TrackRouting {
    TrackRouting {
        main_on: (byte & 0x80) != 0,
        out_select: TrackAudioOut::from_index(byte & 0x7F),
    }
}

#[derive(Clone)]
pub struct PersistentTrackLayout {
    pub state: Arc<AtomicTrackLayout>,
}

impl PersistentTrackLayout {
    pub fn new() -> Self {
        Self {
            state: AtomicTrackLayout::from_state(&TrackLayoutState::default_layout()),
        }
    }
}

impl Default for PersistentTrackLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> nih_plug::params::persist::PersistentField<'a, TrackLayoutState>
    for PersistentTrackLayout
{
    fn set(&self, new_value: TrackLayoutState) {
        self.state.update_from_state(&new_value);
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&TrackLayoutState) -> R,
    {
        let slots = std::array::from_fn(|i| {
            let byte = self.state.slot_kinds[i].load(Ordering::Relaxed);
            let active = byte != 0xFF;
            let kind = if active {
                TrackInstrumentKind::from_index(byte as usize).unwrap_or(TrackInstrumentKind::Kick)
            } else {
                TrackInstrumentKind::Kick
            };
            TrackSlot {
                active,
                name: if active {
                    kind.default_name().to_string()
                } else {
                    String::new()
                },
                kind,
                routing: self.state.routing_for_slot(i),
                midi_note: self.state.slot_midi_notes[i].load(Ordering::Relaxed),
            }
        });
        let state = TrackLayoutState {
            slots,
            global_midi_channel: self.state.global_midi_channel(),
            global_base_note: self.state.global_base_note.load(Ordering::Relaxed),
        };
        f(&state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_is_modular_four_lanes() {
        let layout = TrackLayoutState::default_layout();
        assert_eq!(layout.active_count(), 4);
        assert_eq!(layout.slots[0].kind, TrackInstrumentKind::Kick);
        assert_eq!(layout.slots[1].kind, TrackInstrumentKind::Snare);
        assert_eq!(layout.slots[2].kind, TrackInstrumentKind::HiHat);
        assert_eq!(layout.slots[3].kind, TrackInstrumentKind::Tom);
        assert!(layout.slots[4..].iter().all(|slot| !slot.active));
    }

    #[test]
    fn empty_layout_has_no_active_lanes() {
        let layout = TrackLayoutState::empty_layout();
        assert_eq!(layout.active_count(), 0);
        assert!(layout.slots.iter().all(|slot| !slot.active));
    }

    #[test]
    fn preset_12_layout_uses_core_legacy_kit_without_perc1() {
        let layout = TrackLayoutState::preset_12_layout();
        assert_eq!(layout.active_count(), 12);
        assert_eq!(layout.slots[0].kind, TrackInstrumentKind::Kick);
        assert_eq!(layout.slots[1].kind, TrackInstrumentKind::Snare);
        assert_eq!(layout.slots[2].kind, TrackInstrumentKind::HiHat);
        assert_eq!(layout.slots[3].kind, TrackInstrumentKind::OpenHiHat);
        assert_eq!(layout.slots[4].kind, TrackInstrumentKind::Tom);
        assert_eq!(layout.slots[5].kind, TrackInstrumentKind::Tom);
        assert_eq!(layout.slots[6].kind, TrackInstrumentKind::Tom);
        assert_eq!(layout.slots[11].kind, TrackInstrumentKind::BassDrum808);
        assert!(!layout.slots[12].active);
        assert!(!layout.slots[13].active);
    }

    #[test]
    fn move_slot_preserves_slot_data_and_shifts_intermediate_slots() {
        let mut layout = TrackLayoutState::modular_default_layout();
        layout.slots[0].name = "Custom BD".to_string();
        layout.slots[0].routing.out_select = TrackAudioOut::Out(3);
        layout.slots[0].midi_note = 45;
        layout.move_slot(0, 3);

        assert_eq!(layout.slots[0].kind, TrackInstrumentKind::Snare);
        assert_eq!(layout.slots[1].kind, TrackInstrumentKind::HiHat);
        assert_eq!(layout.slots[2].kind, TrackInstrumentKind::Tom);
        assert_eq!(layout.slots[3].kind, TrackInstrumentKind::Kick);
        assert_eq!(layout.slots[3].name, "Custom BD");
        assert_eq!(layout.slots[3].routing.out_select, TrackAudioOut::Out(3));
        assert_eq!(layout.slots[3].midi_note, 45);

        layout.move_slot(3, 1);
        assert_eq!(layout.slots[0].kind, TrackInstrumentKind::Snare);
        assert_eq!(layout.slots[1].kind, TrackInstrumentKind::Kick);
        assert_eq!(layout.slots[2].kind, TrackInstrumentKind::HiHat);
        assert_eq!(layout.slots[3].kind, TrackInstrumentKind::Tom);
        assert_eq!(layout.slots[1].name, "Custom BD");
    }

    #[test]
    fn legacy_migration_maps_13_voices() {
        let layout = TrackLayoutState::from_legacy_13();
        assert_eq!(layout.active_count(), 13);
        for i in 0..13 {
            assert!(layout.slots[i].active);
        }
        assert!(!layout.slots[13].active);
        assert_eq!(layout.slots[4].kind, TrackInstrumentKind::Tom);
        assert_eq!(layout.slots[5].kind, TrackInstrumentKind::Tom);
        assert_eq!(layout.slots[6].kind, TrackInstrumentKind::Tom);
    }

    #[test]
    fn atomic_roundtrip() {
        let layout = TrackLayoutState::from_legacy_13();
        let atomic = AtomicTrackLayout::from_state(&layout);
        assert_eq!(atomic.kind_for_slot(0), Some(TrackInstrumentKind::Kick));
        assert_eq!(atomic.kind_for_slot(1), Some(TrackInstrumentKind::Snare));
        assert_eq!(atomic.kind_for_slot(12), Some(TrackInstrumentKind::Perc1));
        assert_eq!(atomic.kind_for_slot(13), None);

        let modular = TrackLayoutState::default_layout();
        let atomic = AtomicTrackLayout::from_state(&modular);
        assert_eq!(atomic.kind_for_slot(3), Some(TrackInstrumentKind::Tom));
        assert_eq!(atomic.kind_for_slot(4), None);
    }

    #[test]
    fn assigning_aux_output_is_exclusive_between_slots() {
        let mut layout = TrackLayoutState::default_layout();

        layout.assign_slot_output_exclusive(2, TrackAudioOut::Out(2));
        layout.assign_slot_output_exclusive(3, TrackAudioOut::Out(2));

        assert_eq!(layout.slots[2].routing.out_select, TrackAudioOut::Main);
        assert_eq!(layout.slots[3].routing.out_select, TrackAudioOut::Out(2));
    }

    #[test]
    fn assigning_main_does_not_clear_other_outputs() {
        let mut layout = TrackLayoutState::default_layout();

        layout.assign_slot_output_exclusive(2, TrackAudioOut::Out(2));
        layout.assign_slot_output_exclusive(3, TrackAudioOut::Main);

        assert_eq!(layout.slots[2].routing.out_select, TrackAudioOut::Out(2));
        assert_eq!(layout.slots[3].routing.out_select, TrackAudioOut::Main);
    }
}
