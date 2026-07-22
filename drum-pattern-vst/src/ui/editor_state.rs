//! Editor UI state: clipboard types, popup structs and `EditorUIState` with its
//! lane copy/paste/clear/randomize operations.

use crate::{
    config::GlobalConfig,
    plock::PlockState,
    sequencer::{
        pattern::STEP_COUNT,
        FusedGroup, SharedPattern,
    },
    sound_settings::{SoundSettings, SoundSettingsState},
    track::{TrackInstrumentKind, TrackLayoutState, TrackSlot},
    ui::controls::{set_float_param_if_changed, set_int_param_if_changed},
    DrumFlashParams,
};
use nih_plug::{params::persist::PersistentField, prelude::*};
use nih_plug_egui::egui;
use std::sync::atomic::Ordering;

/// Data stored in the clipboard for lane copy/paste.
#[derive(Debug, Clone)]
pub struct LaneClipboardData {
    /// The instrument kind (Kick, Snare, HiHat, etc.).
    pub kind: TrackInstrumentKind,
    /// The sound settings for this lane (all parameters).
    pub settings: SoundSettings,
    /// Per-slot synthesis algorithm.
    pub algo: i32,
    /// The step sequence for this lane.
    pub steps: [bool; STEP_COUNT],
    /// Step Fusion groups for this lane.
    pub fusions: Vec<FusedGroup>,
    /// Sound plocks for this lane.
    pub sound_plocks: LaneSoundPlocks,
    /// Sequencer plocks for this lane.
    pub seq_plocks: LaneSeqPlocks,
    /// Lane humanize value.
    pub humanize: f32,
    /// Lane push/pull value in ms.
    pub push_pull: f32,
    /// Lane length parameter.
    pub length: i32,
    /// Whether the lane has an individual length lock.
    pub length_locked: bool,
}

#[derive(Debug, Clone)]
pub struct LaneSoundPlocks {
    pub mask: u64,
    pub field_masks: [u64; crate::plock::STEP_COUNT],
    pub values: [[f32; crate::plock::FIELD_COUNT]; crate::plock::STEP_COUNT],
}

#[derive(Debug, Clone)]
pub struct LaneSeqPlocks {
    pub mask: u64,
    pub probabilities: [u32; crate::plock::STEP_COUNT],
    pub stutters: [u32; crate::plock::STEP_COUNT],
    pub conditions: [u32; crate::plock::STEP_COUNT],
    pub microtimings: [u32; crate::plock::STEP_COUNT],
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PlockClipboardEntry {
    pub instrument: usize,
    pub step: usize, // 0-15 within the page
    pub field_mask: u64,
    pub values: Vec<f32>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct FusionClipboardEntry {
    pub instrument: usize,
    pub start_step: usize, // 0-15 within the page
    pub end_step: usize,   // 0-15 within the page
    pub step_count: u8,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PageClipboard {
    pub triggers: [u16; 16],
    pub plocks: Vec<PlockClipboardEntry>,
    #[serde(default)]
    pub fusions: Vec<FusionClipboardEntry>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SinglePlockClipboard {
    pub instrument: usize,
    pub field_mask: u64,
    pub values: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SoundEditorTab {
    #[default]
    Sound,
    Track,
}

#[derive(Clone, Copy, Debug)]
pub struct DragStepState {
    pub slot: usize,
    pub source_step: usize,
    pub source_rect: egui::Rect,
    pub start_time: f64,
    pub active: bool,
    pub current_target: usize,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct EditorUIState {
    pub selected_instrument: usize,
    #[serde(default)]
    pub selected_track_slot: usize,
    #[serde(default)]
    pub sound_editor_tab: SoundEditorTab,
    #[serde(default)]
    pub bottom_panel_tab: usize,
    pub selected_pattern_slot: usize,
    pub last_midi_export_path: Option<String>,
    pub last_midi_export_error: Option<String>,
    pub dump_name_input: String,
    pub current_page: usize, // 0-3 (displaying steps current_page*16 .. current_page*16+15)
    pub follow_mode: bool, // if true, page follows the playhead
    pub page_clipboard: Option<PageClipboard>, // copied page data for paste
    pub plock_clipboard: Option<SinglePlockClipboard>, // copied single plock for paste
    pub sequencer_mode: bool, // if true, right-click opens sequencer params instead of sound plocks
    // Pattern bank state
    /// The slot last loaded into the current pattern (None = fresh/preset).
    pub last_loaded_slot: Option<usize>,
    /// True when the user clicked "Save" and is now selecting a target slot.
    pub save_mode_active: bool,
    /// Copied pattern slot for copy/paste between slots.
    pub pattern_clipboard: Option<crate::pattern_bank::PatternSlot>,
    /// True when the user clicked "Clr" once and needs to confirm.
    pub clear_confirm_mode: bool,
    // Step Fusion state
    /// Start cell of a fusion selection (Shift+click), per track slot.
    pub fusion_selection_start: [Option<usize>; crate::track::MAX_TRACKS],
    /// Currently editing fusion group: (instrument, group_index).
    pub fusion_editing: Option<(usize, usize)>,
    /// True when a fusion group just entered edit mode this frame; requests focus on the step-count field.
    pub fusion_edit_focus_request: bool,
    /// Current value of the track-name field in the Track tab.
    #[serde(skip)]
    pub track_name_input: String,
    /// Slot that `track_name_input` corresponds to; `None` means it needs to be
    /// initialized from the current slot.
    #[serde(skip)]
    pub track_name_input_slot: Option<usize>,
    /// When true, request focus on the Track tab Name field the next time it is drawn.
    #[serde(skip)]
    pub track_name_focus_request: bool,
    /// When set, the current pattern has been edited by the user and should be saved
    /// back to the given pattern-bank slot at the end of the UI frame. Used to keep
    /// edits made while Song Mode is active from being lost when the song advances.
    #[serde(skip)]
    pub pattern_dirty_slot: Option<usize>,
    /// Current value being edited in the Step Fusion step-count field.
    #[serde(skip)]
    pub fusion_edit_steps: u8,
    // Right-click plock popup state (replaces egui context_menu to control frame chrome).
    pub plock_popup: Option<PlockPopup>,
    /// When true, a click occurred while the p-lock popup was open; suppress step-cell
    /// toggles this frame so the popup (not the cell underneath) handles the click.
    #[serde(skip)]
    pub suppress_step_cell_click: bool,
    // Right-click page popup state (Copy/Paste/Clear page).
    pub page_popup: Option<PagePopup>,
    /// Instrument picker for an empty lane (opened by the +N chip).
    #[serde(default)]
    pub add_module_popup: Option<AddModulePopup>,
    /// Pending global lane preset. Requires explicit confirmation because it mutates the current pattern/layout.
    #[serde(default)]
    pub lane_preset_confirm: Option<LanePresetAction>,
    /// Source slot while dragging a lane reorder handle.
    #[serde(default)]
    pub lane_drag_source: Option<usize>,
    /// Selected step in the Song Editor (0..15).
    #[serde(default)]
    pub song_selected_step: usize,
    /// Clipboard for a song step: (slot, repeat).
    #[serde(default)]
    pub song_clipboard: Option<(i8, u8)>,
    /// True when the user clicked "Clear All" and must confirm.
    #[serde(default)]
    pub song_clear_confirm: bool,
    /// Last song sequence published to the audio-thread controller, used to avoid
    /// pushing redundant snapshots every frame.
    #[serde(skip)]
    pub last_published_song: Option<crate::pattern_bank::SongSequence>,
    /// Clipboard for lane copy/paste (instrument + settings + sequence).
    #[serde(skip)]
    pub lane_clipboard: Option<LaneClipboardData>,
    /// Slot waiting for a second click before clearing its grid.
    #[serde(skip)]
    pub lane_clear_grid_confirm: Option<usize>,
    /// Slot waiting for a second click before being deleted/deactivated.
    #[serde(skip)]
    pub lane_delete_confirm: Option<usize>,
    /// Visual flash timer for the Test (T) button when triggered by external MIDI.
    #[serde(skip)]
    pub slot_flash_until: [f64; crate::track::MAX_TRACKS],
    /// Long-press drag state for a step cell (move step with its plocks).
    #[serde(skip)]
    pub step_drag: Option<DragStepState>,
    /// Global user preferences loaded from `Documents/Flash Drum/config.json`.
    #[serde(skip)]
    pub global_config: GlobalConfig,
    /// True when the global settings popup is open.
    #[serde(skip)]
    pub settings_open: bool,
}

pub fn select_legacy_track(state: &mut EditorUIState, slot_idx: usize) {
    let slot_idx = slot_idx.min(crate::track::MAX_TRACKS - 1);
    state.selected_track_slot = slot_idx;
    // `selected_instrument` holds the SLOT index (0..MAX_TRACKS); the voice
    // schema for registry lookups is derived from the slot's kind.
    state.selected_instrument = slot_idx;
}

pub fn voice_idx_for_slot(params: &DrumFlashParams, slot_idx: usize) -> Option<usize> {
    params
        .track_layout
        .state
        .kind_for_slot(slot_idx)
        .map(|k| k.drum_voice_index())
}

/// Voice index used for registry/schema lookups (INSTRUMENTS, special_param, ...)
/// for a slot. Falls back to Kick's schema for inactive/unknown slots so the UI
/// never indexes past `DrumVoice::COUNT`.
pub fn schema_voice_idx(params: &DrumFlashParams, slot_idx: usize) -> usize {
    voice_idx_for_slot(params, slot_idx).unwrap_or(0)
}

pub fn effective_lane_length_for_ui(
    params: &DrumFlashParams,
    slot_idx: usize,
    master_length: usize,
) -> usize {
    // Lane locks and length params are slot-indexed, matching the audio engine
    // (`raw_lengths` / `lane_length_locks.is_locked(slot)` in lib.rs).
    let master_length = master_length.clamp(1, 64);
    if params.lane_length_locks.is_locked(slot_idx) {
        params.lengths()[slot_idx].value().clamp(1, 64) as usize
    } else {
        master_length
    }
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PlockPopup {
    pub instrument: usize,
    pub step: usize,
    /// Trigger state at the moment the popup was opened. P-lock menu clicks must
    /// never toggle the underlying sequencer step.
    #[serde(default)]
    pub step_was_active: bool,
    #[serde(with = "serde_pos2")]
    pub screen_pos: egui::Pos2,
    /// When right-clicking a fused cell, true shows the morph-target submenu.
    #[serde(default)]
    pub morph_menu: bool,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PageMenuAction {
    Paste,
    Clear,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PagePopup {
    pub page: usize,
    #[serde(with = "serde_pos2")]
    pub screen_pos: egui::Pos2,
    pub confirm_action: Option<PageMenuAction>,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LanePresetAction {
    ClearAll,
    Preset4,
    Preset12,
}

impl LanePresetAction {
    pub fn label(self) -> &'static str {
        match self {
            LanePresetAction::ClearAll => "Clear All",
            LanePresetAction::Preset4 => "Preset 4",
            LanePresetAction::Preset12 => "Preset 12",
        }
    }

    pub fn apply_label(self) -> String {
        format!("Apply {}", self.label())
    }
}

impl EditorUIState {
    /// Mark the current pattern as dirty so it will be auto-saved back to the
    /// currently loaded pattern-bank slot at the end of the frame when Song Mode
    /// is active. This prevents edits from being lost when the song advances.
    pub fn mark_pattern_dirty(&mut self) {
        self.pattern_dirty_slot = self.last_loaded_slot;
    }

    /// Copy the current lane (instrument + settings + sequence) to the clipboard.
    pub fn copy_lane(
        &mut self,
        params: &DrumFlashParams,
        slot: usize,
        settings_state: &SoundSettingsState,
        pattern: &SharedPattern,
        plock: &PlockState,
    ) {
        let Some(kind) = params.track_layout.state.kind_for_slot(slot) else {
            return;
        };

        let settings = settings_state.get_settings_for_slot(slot);
        let algo = params.algos()[slot].value();
        let steps = std::array::from_fn(|step| pattern.is_active(step, slot));
        let fusions = pattern.load_fusions(slot);
        let sound_plocks = copy_lane_sound_plocks(plock, slot);
        let seq_plocks = copy_lane_seq_plocks(&params.seq_plock_state.state, slot);
        let humanize = params.humanizes()[slot].value();
        let push_pull = params.pushes()[slot].value();
        let length = params.lengths()[slot].value();
        let length_locked = params.lane_length_locks.is_locked(slot);

        self.lane_clipboard = Some(LaneClipboardData {
            kind,
            settings,
            algo,
            steps,
            fusions,
            sound_plocks,
            seq_plocks,
            humanize,
            push_pull,
            length,
            length_locked,
        });
    }

    /// Paste the full clipboard data to the target lane.
    /// Returns true if the paste was successful.
    pub fn paste_lane(
        &mut self,
        setter: &ParamSetter,
        params: &DrumFlashParams,
        target_slot: usize,
        settings_state: &SoundSettingsState,
        pattern: &SharedPattern,
        plock: &PlockState,
    ) -> bool {
        if target_slot >= crate::track::MAX_TRACKS {
            return false;
        }

        let Some(clipboard) = &self.lane_clipboard else {
            return false;
        };

        let mut layout =
            PersistentField::<TrackLayoutState>::map(&params.track_layout, |s| s.clone());
        if layout.slots[target_slot].active {
            layout.slots[target_slot].kind = clipboard.kind;
            layout.slots[target_slot].name = clipboard.kind.default_name().to_string();
            layout.slots[target_slot].midi_note = clipboard.kind.default_midi_note();
        } else {
            layout.slots[target_slot] = TrackSlot::active_with_kind(clipboard.kind);
        }
        PersistentField::<TrackLayoutState>::set(&params.track_layout, layout);

        settings_state.set_settings_for_slot(target_slot, &clipboard.settings);
        set_int_param_if_changed(setter, params.algos()[target_slot], clipboard.algo);

        paste_lane_steps(pattern, target_slot, &clipboard.steps);
        pattern.store_fusions(target_slot, &clipboard.fusions);
        paste_lane_sound_plocks(plock, target_slot, &clipboard.sound_plocks);
        paste_lane_seq_plocks(
            &params.seq_plock_state.state,
            target_slot,
            &clipboard.seq_plocks,
        );
        set_float_param_if_changed(setter, params.humanizes()[target_slot], clipboard.humanize);
        set_float_param_if_changed(setter, params.pushes()[target_slot], clipboard.push_pull);
        set_int_param_if_changed(setter, params.lengths()[target_slot], clipboard.length);
        set_lane_length_lock_for_slot(params, target_slot, clipboard.length_locked);

        select_legacy_track(self, target_slot);
        self.mark_pattern_dirty();
        true
    }

    /// Paste only the visible step grid to an already active target lane.
    /// Instrument, sound settings, plocks, fusions, routing and lane controls stay untouched.
    pub fn paste_grid(
        &mut self,
        params: &DrumFlashParams,
        target_slot: usize,
        pattern: &SharedPattern,
    ) -> bool {
        if target_slot >= crate::track::MAX_TRACKS {
            return false;
        }
        if params
            .track_layout
            .state
            .kind_for_slot(target_slot)
            .is_none()
        {
            return false;
        }

        let Some(clipboard) = &self.lane_clipboard else {
            return false;
        };

        paste_lane_steps(pattern, target_slot, &clipboard.steps);
        select_legacy_track(self, target_slot);
        self.mark_pattern_dirty();
        true
    }

    /// Clear the musical grid for an active lane, keeping the module and settings intact.
    pub fn clear_lane(
        &mut self,
        params: &DrumFlashParams,
        slot: usize,
        pattern: &SharedPattern,
        plock: &PlockState,
    ) -> bool {
        if slot >= crate::track::MAX_TRACKS || !params.track_layout.state.is_active(slot) {
            return false;
        }

        clear_grid_steps(pattern, slot);
        pattern.store_fusions(slot, &[]);
        clear_grid_sound_plocks(plock, slot);
        clear_grid_seq_plocks(&params.seq_plock_state.state, slot);

        self.fusion_selection_start[slot] = None;
        self.fusion_editing = self.fusion_editing.filter(|(idx, _)| *idx != slot);
        self.plock_popup = self.plock_popup.filter(|popup| popup.instrument != slot);
        self.lane_clear_grid_confirm = None;
        self.lane_delete_confirm = None;
        self.mark_pattern_dirty();
        true
    }

    /// Randomize the musical grid for an active lane, keeping the module and settings intact.
    pub fn randomize_lane(
        &mut self,
        params: &DrumFlashParams,
        slot: usize,
        pattern: &SharedPattern,
        plock: &PlockState,
    ) -> bool {
        if slot >= crate::track::MAX_TRACKS || !params.track_layout.state.is_active(slot) {
            return false;
        }

        clear_grid_steps(pattern, slot);
        pattern.store_fusions(slot, &[]);
        clear_grid_sound_plocks(plock, slot);
        clear_grid_seq_plocks(&params.seq_plock_state.state, slot);
        randomize_lane_steps(pattern, slot, 0.3);

        self.fusion_selection_start[slot] = None;
        self.fusion_editing = self.fusion_editing.filter(|(idx, _)| *idx != slot);
        self.plock_popup = self.plock_popup.filter(|popup| popup.instrument != slot);
        self.lane_clear_grid_confirm = None;
        self.lane_delete_confirm = None;
        self.mark_pattern_dirty();
        true
    }
}

pub fn copy_lane_sound_plocks(plock: &PlockState, slot: usize) -> LaneSoundPlocks {
    LaneSoundPlocks {
        mask: plock.masks.masks[slot].load(Ordering::Relaxed),
        field_masks: std::array::from_fn(|step| plock.field_masks.get_raw(slot, step)),
        values: std::array::from_fn(|step| {
            std::array::from_fn(|field| plock.values.get(slot, step, field))
        }),
    }
}

pub fn paste_lane_sound_plocks(plock: &PlockState, target_slot: usize, data: &LaneSoundPlocks) {
    plock.masks.masks[target_slot].store(data.mask, Ordering::Relaxed);
    for step in 0..crate::plock::STEP_COUNT {
        plock
            .field_masks
            .set_raw(target_slot, step, data.field_masks[step]);
        for field in 0..crate::plock::FIELD_COUNT {
            plock
                .values
                .set(target_slot, step, field, data.values[step][field]);
        }
    }
}

pub fn clear_grid_sound_plocks(plock: &PlockState, slot: usize) {
    if slot >= crate::track::MAX_TRACKS {
        return;
    }
    plock.masks.masks[slot].store(0, Ordering::Relaxed);
    for step in 0..crate::plock::STEP_COUNT {
        plock.field_masks.set_raw(slot, step, 0);
        for field in 0..crate::plock::FIELD_COUNT {
            plock.values.set(slot, step, field, 0.0);
        }
    }
}

pub fn copy_lane_seq_plocks(seq: &crate::plock::SequencerPlockState, slot: usize) -> LaneSeqPlocks {
    LaneSeqPlocks {
        mask: seq.masks[slot].load(Ordering::Relaxed),
        probabilities: std::array::from_fn(|step| {
            seq.probabilities[slot][step].load(Ordering::Relaxed)
        }),
        stutters: std::array::from_fn(|step| seq.stutters[slot][step].load(Ordering::Relaxed)),
        conditions: std::array::from_fn(|step| seq.conditions[slot][step].load(Ordering::Relaxed)),
        microtimings: std::array::from_fn(|step| {
            seq.microtimings[slot][step].load(Ordering::Relaxed)
        }),
    }
}

pub fn paste_lane_seq_plocks(
    seq: &crate::plock::SequencerPlockState,
    target_slot: usize,
    data: &LaneSeqPlocks,
) {
    seq.masks[target_slot].store(data.mask, Ordering::Relaxed);
    for step in 0..crate::plock::STEP_COUNT {
        seq.probabilities[target_slot][step].store(data.probabilities[step], Ordering::Relaxed);
        seq.stutters[target_slot][step].store(data.stutters[step], Ordering::Relaxed);
        seq.conditions[target_slot][step].store(data.conditions[step], Ordering::Relaxed);
        seq.microtimings[target_slot][step].store(data.microtimings[step], Ordering::Relaxed);
    }
}

pub fn clear_grid_seq_plocks(seq: &crate::plock::SequencerPlockState, slot: usize) {
    if slot >= crate::track::MAX_TRACKS {
        return;
    }
    seq.masks[slot].store(0, Ordering::Relaxed);
    for step in 0..crate::plock::STEP_COUNT {
        seq.probabilities[slot][step].store(f32::to_bits(1.0), Ordering::Relaxed);
        seq.stutters[slot][step].store(f32::to_bits(1.0), Ordering::Relaxed);
        seq.conditions[slot][step].store(0, Ordering::Relaxed);
        seq.microtimings[slot][step].store(0, Ordering::Relaxed);
    }
}

pub fn set_lane_length_lock_for_slot(params: &DrumFlashParams, slot: usize, locked: bool) {
    if slot >= crate::track::MAX_TRACKS {
        return;
    }
    let old_mask = PersistentField::<u16>::map(&params.lane_length_locks, |mask| *mask);
    let bit = 1u16 << slot;
    let new_mask = if locked {
        old_mask | bit
    } else {
        old_mask & !bit
    };
    if new_mask != old_mask {
        PersistentField::<u16>::set(&params.lane_length_locks, new_mask);
    }
}

pub fn paste_lane_steps(pattern: &SharedPattern, target_slot: usize, steps: &[bool; STEP_COUNT]) {
    if target_slot >= crate::track::MAX_TRACKS {
        return;
    }
    let bit = 1u16 << target_slot;
    for (step, active) in steps.iter().copied().enumerate() {
        let mask = pattern.load_step_mask(step);
        let next = if active { mask | bit } else { mask & !bit };
        if next != mask {
            pattern.set_step_mask(step, next);
        }
    }
}

pub fn clear_grid_steps(pattern: &SharedPattern, target_slot: usize) {
    if target_slot >= crate::track::MAX_TRACKS {
        return;
    }
    let bit = 1u16 << target_slot;
    for step in 0..STEP_COUNT {
        let mask = pattern.load_step_mask(step);
        let next = mask & !bit;
        if next != mask {
            pattern.set_step_mask(step, next);
        }
    }
}

pub fn randomize_lane_steps(pattern: &SharedPattern, target_slot: usize, density: f32) {
    if target_slot >= crate::track::MAX_TRACKS {
        return;
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);
    let mut rng = seed;
    let bit = 1u16 << target_slot;
    for step in 0..STEP_COUNT {
        let mask = pattern.load_step_mask(step);
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        let active = (rng % 1000) < (density * 1000.0) as u64;
        let next = if active { mask | bit } else { mask & !bit };
        if next != mask {
            pattern.set_step_mask(step, next);
        }
    }
}

/// Instrument picker for an empty lane (opened by the `+N` chip).
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct AddModulePopup {
    pub slot: usize,
    #[serde(with = "serde_pos2")]
    pub screen_pos: egui::Pos2,
}

pub mod serde_pos2 {
    use nih_plug_egui::egui::Pos2;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(pos: &Pos2, s: S) -> Result<S::Ok, S::Error> {
        [pos.x, pos.y].serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Pos2, D::Error> {
        let [x, y] = <[f32; 2]>::deserialize(d)?;
        Ok(Pos2::new(x, y))
    }
}
