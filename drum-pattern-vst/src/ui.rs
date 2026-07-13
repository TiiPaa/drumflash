use nih_plug::{params::persist::PersistentField, prelude::*};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32, RichText, Vec2},
    resizable_window::ResizableWindow,
};
use std::{
    fs::create_dir_all,
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
        Arc,
    },
};

use crate::{
    generator::{self, GeneratorType, Style},
    midi_export,
    pattern_bank::{SLOT_COUNT, SONG_BLOCKS},
    plock::PlockState,
    preset_dumps,
    sequencer::{pattern::STEP_COUNT, FusedGroup, MorphTarget, Pattern, SharedPattern},
    sound_settings::{SoundSettings, SoundSettingsState},
    synthesis::{self, DrumVoice, VoiceSettings},
    track::{TrackInstrumentKind, TrackLayoutState, TrackSlot},
    DrumFlashParams, BUILD_ID,
};

/// Data stored in the clipboard for lane copy/paste.
#[derive(Debug, Clone)]
struct LaneClipboardData {
    /// The instrument kind (Kick, Snare, HiHat, etc.).
    kind: TrackInstrumentKind,
    /// The sound settings for this lane (all parameters).
    settings: SoundSettings,
    /// Per-slot synthesis algorithm.
    algo: i32,
    /// The step sequence for this lane.
    steps: [bool; STEP_COUNT],
    /// Step Fusion groups for this lane.
    fusions: Vec<FusedGroup>,
    /// Sound plocks for this lane.
    sound_plocks: LaneSoundPlocks,
    /// Sequencer plocks for this lane.
    seq_plocks: LaneSeqPlocks,
    /// Lane humanize value.
    humanize: f32,
    /// Lane push/pull value in ms.
    push_pull: f32,
    /// Lane length parameter.
    length: i32,
    /// Whether the lane has an individual length lock.
    length_locked: bool,
}

#[derive(Debug, Clone)]
struct LaneSoundPlocks {
    mask: u64,
    field_masks: [u64; crate::plock::STEP_COUNT],
    values: [[f32; crate::plock::FIELD_COUNT]; crate::plock::STEP_COUNT],
}

#[derive(Debug, Clone)]
struct LaneSeqPlocks {
    mask: u64,
    probabilities: [u32; crate::plock::STEP_COUNT],
    stutters: [u32; crate::plock::STEP_COUNT],
    conditions: [u32; crate::plock::STEP_COUNT],
    microtimings: [u32; crate::plock::STEP_COUNT],
}

mod envelope_viz;
mod local_param_slider;
mod theme;
mod widgets;

use envelope_viz::{draw_amp_envelope, draw_filter_envelope};
use local_param_slider::LocalParamSlider;
use theme::*;
use widgets::*;

// ---------------------------------------------------------------------------------------------------------------
// Frequency / Note conversion utilities
// ---------------------------------------------------------------------------------------------------------------
const EDITOR_LABEL_W: f32 = 138.0;
const EDITOR_PARAMS_W: f32 = 340.0;
const EDITOR_VALUE_W: f32 = 52.0;

fn freq_to_note(freq: f32) -> f32 {
    69.0 + 12.0 * (freq / 440.0).log2()
}

fn note_to_freq(note: f32) -> f32 {
    440.0 * 2.0f32.powf((note - 69.0) / 12.0)
}

fn format_value_for_plock(
    field: crate::instrument_registry::StandardField,
    value: f32,
    min: f32,
    max: f32,
) -> String {
    match field {
        crate::instrument_registry::StandardField::Volume => format!("{:.2}", value),
        crate::instrument_registry::StandardField::Analog
        | crate::instrument_registry::StandardField::Stereo => format!("{:.2}", value),
        crate::instrument_registry::StandardField::Decay
        | crate::instrument_registry::StandardField::Release
        | crate::instrument_registry::StandardField::Attack
        | crate::instrument_registry::StandardField::Hold
        | crate::instrument_registry::StandardField::FilterEnvDecay => format!("{:.2}", value),
        crate::instrument_registry::StandardField::DecayCurve
        | crate::instrument_registry::StandardField::ReleaseCurve
        | crate::instrument_registry::StandardField::FilterEnvAmount => format!("{:.2}", value),
        crate::instrument_registry::StandardField::Freq
        | crate::instrument_registry::StandardField::FilterFreq => {
            let range = max - min;
            if range >= 1000.0 || max >= 1000.0 {
                format!("{:.1}", value)
            } else {
                format!("{:.2}", value)
            }
        }
    }
}

fn format_value_for_plock_special(value: f32, min: f32, max: f32) -> String {
    let range = max - min;
    if range >= 1000.0 || max >= 1000.0 {
        format!("{:.1}", value)
    } else if range <= 1.0 {
        format!("{:.2}", value)
    } else {
        format!("{:.2}", value)
    }
}

fn note_name(note: f32) -> String {
    let note = note.round() as i32;
    let octave = (note / 12) - 1;
    let note_idx = note % 12;
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", names[note_idx as usize], octave)
}

fn plock_menu_frame(ui: &mut egui::Ui, accent: Color32, content: impl FnOnce(&mut egui::Ui)) {
    // Remove the default context-menu border/shadow so our inner frame is the only chrome.
    ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    ui.visuals_mut().widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);

    let frame = egui::Frame::NONE
        .fill(P_ACTIVE)
        .corner_radius(RADIUS_PANEL)
        .inner_margin(egui::Margin::same(12));
    frame.show(ui, |ui| {
        ui.set_min_width(280.0);
        ui.set_max_width(350.0);
        // Top accent bar
        let bar_rect = ui.available_rect_before_wrap();
        let bar_rect = egui::Rect::from_min_max(
            bar_rect.left_top(),
            egui::pos2(bar_rect.right(), bar_rect.top() + 3.0),
        );
        ui.painter().rect_filled(bar_rect, 0.0, accent);
        ui.add_space(8.0);
        content(ui);
    });
}

fn page_menu_frame(ui: &mut egui::Ui, accent: Color32, content: impl FnOnce(&mut egui::Ui)) {
    ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    ui.visuals_mut().widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);

    let frame = egui::Frame::NONE
        .fill(P_ACTIVE)
        .corner_radius(RADIUS_PANEL)
        .inner_margin(egui::Margin::same(10));
    frame.show(ui, |ui| {
        ui.set_min_width(130.0);
        ui.set_max_width(150.0);
        let bar_rect = ui.available_rect_before_wrap();
        let bar_rect = egui::Rect::from_min_max(
            bar_rect.left_top(),
            egui::pos2(bar_rect.right(), bar_rect.top() + 3.0),
        );
        ui.painter().rect_filled(bar_rect, 0.0, accent);
        ui.add_space(8.0);
        content(ui);
    });
}

fn plock_menu_header(ui: &mut egui::Ui, title: &str, _step: usize, accent: Color32) -> bool {
    let mut close_clicked = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).font(f_sans_sb(11.0)).color(accent));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button_size = egui::Vec2::new(22.0, 22.0);
            let response = ui.allocate_response(button_size, egui::Sense::click());
            let pressed = response.is_pointer_button_down_on();

            let (fill, text_color) = if pressed {
                (accent, INK)
            } else {
                (Color32::TRANSPARENT, INK3)
            };

            if pressed {
                ui.painter().rect_filled(response.rect, RADIUS_CTL, fill);
            }
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                f_sans_med(14.0),
                text_color,
            );

            if response.clicked() {
                close_clicked = true;
            }
        });
    });
    ui.add_space(4.0);
    close_clicked
}

fn page_menu_header(ui: &mut egui::Ui, title: &str, accent: Color32) {
    ui.label(RichText::new(title).font(f_sans_sb(11.0)).color(accent));
    ui.add_space(4.0);
}

fn plock_menu_row(
    ui: &mut egui::Ui,
    label: &str,
    accent: Color32,
    overridden: bool,
    value_text: Option<&str>,
    content: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.set_height(22.0);
        let label_color = if overridden { accent } else { INK3 };
        ui.label(
            RichText::new(label)
                .font(f_sans_med(10.5))
                .color(label_color),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let response = content(ui);
            if let Some(text) = value_text {
                ui.add_space(8.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(48.0, 22.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label(RichText::new(text).font(f_mono_med(10.0)).color(INK2));
                    },
                );
            }
            response
        })
        .inner
    })
    .inner
}

fn plock_menu_action_row(ui: &mut egui::Ui, label: &str, accent: Color32) -> egui::Response {
    ui.add_sized(
        Vec2::new(ui.available_width(), 26.0),
        egui::Button::new(RichText::new(label).font(f_sans_med(10.5)).color(accent))
            .fill(PANEL2)
            .stroke(egui::Stroke::new(1.0, LINE2))
            .corner_radius(6.0),
    )
}

fn plock_menu_enum_row(
    ui: &mut egui::Ui,
    label: &str,
    accent: Color32,
    overridden: bool,
    current_value: f32,
    options: &[&str],
    id_salt: &str,
) -> (egui::Response, Option<f32>) {
    let current_idx = (current_value as usize).min(options.len().saturating_sub(1));
    let value_text = options[current_idx];
    let mut picked = None;
    let response = plock_menu_row(ui, label, accent, overridden, Some(value_text), |ui| {
        let (resp, p) = styled_select(ui, id_salt, current_idx, options, 120.0);
        if let Some(p) = p {
            picked = Some(p as f32);
        }
        resp
    });
    (response, picked)
}

fn install_egui_fonts(ctx: &egui::Context) {
    use egui::FontFamily;
    let mut fonts = egui::FontDefinitions::default();

    // Default fallback chains (emoji / missing-glyph coverage) kept after our faces.
    let prop_fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mono_fallback = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    // IBM Plex faces — matches the design's weight scale (Sans 400/500/600/700, Mono 400/500/600).
    let faces: [(&str, &[u8]); 7] = [
        (
            "sans_400",
            include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf"),
        ),
        (
            "sans_500",
            include_bytes!("../assets/fonts/IBMPlexSans-Medium.ttf"),
        ),
        (
            "sans_600",
            include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf"),
        ),
        (
            "sans_700",
            include_bytes!("../assets/fonts/IBMPlexSans-Bold.ttf"),
        ),
        (
            "mono_400",
            include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf"),
        ),
        (
            "mono_500",
            include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf"),
        ),
        (
            "mono_600",
            include_bytes!("../assets/fonts/IBMPlexMono-SemiBold.ttf"),
        ),
    ];
    for (name, bytes) in faces {
        fonts.font_data.insert(
            name.to_string(),
            Arc::new(egui::FontData::from_static(bytes)),
        );
    }

    let with_fallback = |face: &str, fallback: &[String]| {
        let mut v = vec![face.to_string()];
        v.extend_from_slice(fallback);
        v
    };

    // Built-in families default to Regular (with the original fallbacks appended).
    fonts.families.insert(
        FontFamily::Proportional,
        with_fallback("sans_400", &prop_fallback),
    );
    fonts.families.insert(
        FontFamily::Monospace,
        with_fallback("mono_400", &mono_fallback),
    );

    // Named weight families — used via FontId::new(size, FontFamily::Name(..)).
    let named: [(&str, &str, &[String]); 5] = [
        ("sans_med", "sans_500", &prop_fallback),
        ("sans_sb", "sans_600", &prop_fallback),
        ("sans_bold", "sans_700", &prop_fallback),
        ("mono_med", "mono_500", &mono_fallback),
        ("mono_sb", "mono_600", &mono_fallback),
    ];
    for (alias, face, fb) in named {
        fonts
            .families
            .insert(FontFamily::Name(alias.into()), with_fallback(face, fb));
    }

    ctx.set_fonts(fonts);
}

fn draw_track_length_control(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    length_param: &IntParam,
    instrument: usize,
    master_length: usize,
) -> bool {
    let locked = params.lane_length_locks.is_locked(instrument);
    let raw = length_param.value() as usize;
    let mut length_value = if locked {
        raw as i32
    } else {
        master_length as i32
    };

    let response = ui.add_sized(
        Vec2::new(35.0, 20.0),
        egui::DragValue::new(&mut length_value)
            .speed(1.0)
            .range(1..=64),
    );
    let changed = response.changed();
    let response = response.on_hover_text(if locked {
        "Locked lane length. Right-click to follow pattern length."
    } else {
        "Follows pattern length. Drag to lock this lane."
    });
    let interacted =
        changed || response.clicked() || response.dragged() || response.secondary_clicked();

    response.context_menu(|ui| {
        if locked {
            if ui.button("Follow pattern length").clicked() {
                params.lane_length_locks.set_locked(instrument, false);
                setter.set_parameter(length_param, master_length as i32);
                ui.close_menu();
            }
        } else {
            ui.label("Already follows pattern length");
        }
    });

    if changed {
        params.lane_length_locks.set_locked(instrument, true);
        setter.set_parameter(length_param, length_value.clamp(1, 64));
    }

    interacted
}

fn fusion_modifier_pressed(ui: &egui::Ui) -> bool {
    ui.input(|i| i.modifiers.shift) || platform_shift_pressed()
}

#[cfg(target_os = "windows")]
fn platform_shift_pressed() -> bool {
    const VK_SHIFT: i32 = 0x10;
    const VK_LSHIFT: i32 = 0xA0;
    const VK_RSHIFT: i32 = 0xA1;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetAsyncKeyState(vkey: i32) -> i16;
    }

    unsafe {
        [VK_SHIFT, VK_LSHIFT, VK_RSHIFT]
            .iter()
            .any(|&key| (GetAsyncKeyState(key) as u16 & 0x8000) != 0)
    }
}

#[cfg(not(target_os = "windows"))]
fn platform_shift_pressed() -> bool {
    false
}

// Instrument labels and names are sourced from instrument_registry::INSTRUMENTS

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PlockClipboardEntry {
    instrument: usize,
    step: usize, // 0-15 within the page
    field_mask: u64,
    values: Vec<f32>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct FusionClipboardEntry {
    instrument: usize,
    start_step: usize, // 0-15 within the page
    end_step: usize,   // 0-15 within the page
    step_count: u8,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PageClipboard {
    triggers: [u16; 16],
    plocks: Vec<PlockClipboardEntry>,
    #[serde(default)]
    fusions: Vec<FusionClipboardEntry>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SinglePlockClipboard {
    instrument: usize,
    field_mask: u64,
    values: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
enum SoundEditorTab {
    #[default]
    Sound,
    Track,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct EditorUIState {
    selected_instrument: usize,
    #[serde(default)]
    selected_track_slot: usize,
    #[serde(default)]
    sound_editor_tab: SoundEditorTab,
    #[serde(default)]
    bottom_panel_tab: usize,
    selected_pattern_slot: usize,
    last_midi_export_path: Option<String>,
    last_midi_export_error: Option<String>,
    dump_name_input: String,
    current_page: usize, // 0-3 (displaying steps current_page*16 .. current_page*16+15)
    follow_mode: bool,   // if true, page follows the playhead
    page_clipboard: Option<PageClipboard>, // copied page data for paste
    plock_clipboard: Option<SinglePlockClipboard>, // copied single plock for paste
    sequencer_mode: bool, // if true, right-click opens sequencer params instead of sound plocks
    // Pattern bank state
    /// The slot last loaded into the current pattern (None = fresh/preset).
    last_loaded_slot: Option<usize>,
    /// True when the user clicked "Save" and is now selecting a target slot.
    save_mode_active: bool,
    /// Copied pattern slot for copy/paste between slots.
    pattern_clipboard: Option<crate::pattern_bank::PatternSlot>,
    /// True when the user clicked "Clr" once and needs to confirm.
    clear_confirm_mode: bool,
    // Step Fusion state
    /// Start cell of a fusion selection (Shift+click), per track slot.
    fusion_selection_start: [Option<usize>; crate::track::MAX_TRACKS],
    /// Currently editing fusion group: (instrument, group_index).
    fusion_editing: Option<(usize, usize)>,
    /// True when a fusion group just entered edit mode this frame; requests focus on the step-count field.
    fusion_edit_focus_request: bool,
    /// Current value being edited in the Step Fusion step-count field.
    #[serde(skip)]
    fusion_edit_steps: u8,
    // Right-click plock popup state (replaces egui context_menu to control frame chrome).
    plock_popup: Option<PlockPopup>,
    // Right-click page popup state (Copy/Paste/Clear page).
    page_popup: Option<PagePopup>,
    /// Instrument picker for an empty lane (opened by the +N chip).
    #[serde(default)]
    add_module_popup: Option<AddModulePopup>,
    /// Pending global lane preset. Requires explicit confirmation because it mutates the current pattern/layout.
    #[serde(default)]
    lane_preset_confirm: Option<LanePresetAction>,
    /// Source slot while dragging a lane reorder handle.
    #[serde(default)]
    lane_drag_source: Option<usize>,
    /// Selected step in the Song Editor (0..15).
    #[serde(default)]
    song_selected_step: usize,
    /// Clipboard for a song step: (slot, repeat).
    #[serde(default)]
    song_clipboard: Option<(i8, u8)>,
    /// True when the user clicked "Clear All" and must confirm.
    #[serde(default)]
    song_clear_confirm: bool,
    /// Clipboard for lane copy/paste (instrument + settings + sequence).
    #[serde(skip)]
    lane_clipboard: Option<LaneClipboardData>,
    /// Slot waiting for a second click before clearing its grid.
    #[serde(skip)]
    lane_clear_grid_confirm: Option<usize>,
    /// Visual flash timer for the Test (T) button when triggered by external MIDI.
    #[serde(skip)]
    slot_flash_until: [f64; crate::track::MAX_TRACKS],
}

fn select_legacy_track(state: &mut EditorUIState, slot_idx: usize) {
    let slot_idx = slot_idx.min(crate::track::MAX_TRACKS - 1);
    state.selected_track_slot = slot_idx;
    // `selected_instrument` holds the SLOT index (0..MAX_TRACKS); the voice
    // schema for registry lookups is derived from the slot's kind.
    state.selected_instrument = slot_idx;
}

fn voice_idx_for_slot(params: &DrumFlashParams, slot_idx: usize) -> Option<usize> {
    params
        .track_layout
        .state
        .kind_for_slot(slot_idx)
        .map(|k| k.drum_voice_index())
}

/// Voice index used for registry/schema lookups (INSTRUMENTS, special_param, ...)
/// for a slot. Falls back to Kick's schema for inactive/unknown slots so the UI
/// never indexes past `DrumVoice::COUNT`.
fn schema_voice_idx(params: &DrumFlashParams, slot_idx: usize) -> usize {
    voice_idx_for_slot(params, slot_idx).unwrap_or(0)
}

fn effective_lane_length_for_ui(
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
struct PlockPopup {
    instrument: usize,
    step: usize,
    #[serde(with = "serde_pos2")]
    screen_pos: egui::Pos2,
    /// When right-clicking a fused cell, true shows the morph-target submenu.
    #[serde(default)]
    morph_menu: bool,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum PageMenuAction {
    Paste,
    Clear,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct PagePopup {
    page: usize,
    #[serde(with = "serde_pos2")]
    screen_pos: egui::Pos2,
    confirm_action: Option<PageMenuAction>,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum LanePresetAction {
    ClearAll,
    Preset4,
    Preset12,
}

impl LanePresetAction {
    fn label(self) -> &'static str {
        match self {
            LanePresetAction::ClearAll => "Clear All",
            LanePresetAction::Preset4 => "Preset 4",
            LanePresetAction::Preset12 => "Preset 12",
        }
    }

    fn apply_label(self) -> String {
        format!("Apply {}", self.label())
    }
}

impl EditorUIState {
    /// Copy the current lane (instrument + settings + sequence) to the clipboard.
    fn copy_lane(
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
    fn paste_lane(
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
        true
    }

    /// Paste only the visible step grid to an already active target lane.
    /// Instrument, sound settings, plocks, fusions, routing and lane controls stay untouched.
    fn paste_grid(
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
        true
    }

    /// Clear the musical grid for an active lane, keeping the module and settings intact.
    fn clear_grid(
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
        select_legacy_track(self, slot);
        true
    }
}

fn copy_lane_sound_plocks(plock: &PlockState, slot: usize) -> LaneSoundPlocks {
    LaneSoundPlocks {
        mask: plock.masks.masks[slot].load(Ordering::Relaxed),
        field_masks: std::array::from_fn(|step| plock.field_masks.get_raw(slot, step)),
        values: std::array::from_fn(|step| {
            std::array::from_fn(|field| plock.values.get(slot, step, field))
        }),
    }
}

fn paste_lane_sound_plocks(plock: &PlockState, target_slot: usize, data: &LaneSoundPlocks) {
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

fn clear_grid_sound_plocks(plock: &PlockState, slot: usize) {
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

fn copy_lane_seq_plocks(seq: &crate::plock::SequencerPlockState, slot: usize) -> LaneSeqPlocks {
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

fn paste_lane_seq_plocks(
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

fn clear_grid_seq_plocks(seq: &crate::plock::SequencerPlockState, slot: usize) {
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

fn set_lane_length_lock_for_slot(params: &DrumFlashParams, slot: usize, locked: bool) {
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

fn paste_lane_steps(pattern: &SharedPattern, target_slot: usize, steps: &[bool; STEP_COUNT]) {
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

fn clear_grid_steps(pattern: &SharedPattern, target_slot: usize) {
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

/// Instrument picker for an empty lane (opened by the `+N` chip).
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct AddModulePopup {
    slot: usize,
    #[serde(with = "serde_pos2")]
    screen_pos: egui::Pos2,
}

mod serde_pos2 {
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

pub fn create_editor(
    params: Arc<DrumFlashParams>,
    current_step: Arc<AtomicU32>,
    current_steps: Arc<[AtomicU32; crate::track::MAX_TRACKS]>,
    pattern: Arc<SharedPattern>,
    voice_test_triggers: Arc<[AtomicBool; crate::track::MAX_TRACKS]>,
    external_midi_triggers: Arc<[AtomicBool; crate::track::MAX_TRACKS]>,
    sound_settings_state: Arc<SoundSettingsState>,
    plock_state: Arc<PlockState>,
    save_pattern_request: Arc<AtomicU32>,
    load_pattern_request: Arc<AtomicU32>,
    song_mode: Arc<AtomicBool>,
    song_position: Arc<AtomicU32>,
    pending_pattern_length: Arc<AtomicI32>,
    audio_last_loaded_slot: Arc<AtomicU32>,
    clear_plocks_request: Arc<AtomicBool>,
) -> Option<Box<dyn Editor>> {
    let params_for_ui = params.clone();
    let editor_state = params.editor_state.clone();
    let pattern_for_ui = pattern.clone();
    let voice_test_triggers_for_ui = voice_test_triggers.clone();
    let external_midi_triggers_for_ui = external_midi_triggers.clone();
    let sound_settings_for_ui = sound_settings_state.clone();
    let current_steps_for_ui = current_steps.clone();
    let plock_for_ui = plock_state.clone();
    let song_mode_for_ui = song_mode.clone();
    let song_position_for_ui = song_position.clone();
    let pending_pattern_length_for_ui = pending_pattern_length.clone();
    let audio_last_loaded_slot_for_ui = audio_last_loaded_slot.clone();
    let clear_plocks_request_for_ui = clear_plocks_request.clone();

    create_egui_editor(
        params.editor_state.clone(),
        EditorUIState::default(),
        |egui_ctx, _state| {
            install_egui_fonts(egui_ctx);

            // Style global sombre
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = BG;
            visuals.window_fill = BG;
            visuals.extreme_bg_color = BG;
            visuals.widgets.inactive.bg_fill = PANEL2;
            visuals.widgets.hovered.bg_fill = P_HOVER;
            visuals.widgets.active.bg_fill = P_ACTIVE;
            visuals.selection.bg_fill = BLUE;
            visuals.faint_bg_color = PANEL;
            visuals.extreme_bg_color = BG;
            visuals.window_stroke = egui::Stroke::NONE;
            visuals.popup_shadow = egui::epaint::Shadow::NONE;
            visuals.menu_corner_radius = egui::CornerRadius::same(RADIUS_PANEL as u8);
            visuals.widgets.noninteractive.bg_fill = BG;

            // Chrome tokens: rounded corners, hairline strokes, no hover-expansion.
            let cr = egui::CornerRadius::same(6);
            visuals.widgets.noninteractive.corner_radius = cr;
            visuals.widgets.inactive.corner_radius = cr;
            visuals.widgets.hovered.corner_radius = cr;
            visuals.widgets.active.corner_radius = cr;
            visuals.widgets.open.corner_radius = cr;
            visuals.widgets.inactive.expansion = 0.0;
            visuals.widgets.hovered.expansion = 0.0;
            visuals.widgets.active.expansion = 0.0;
            visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, LINE);
            visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, LINE2);
            visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, BLUE);
            visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, BLUE);
            visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, LINE2);
            visuals.selection.stroke = egui::Stroke::new(1.0, BLUE);
            egui_ctx.set_visuals(visuals);
        },
        move |egui_ctx, setter, state| {
            #[cfg(target_os = "windows")]
            nih_plug_egui::set_keyboard_focus(egui_ctx.wants_keyboard_input());

            // Apply any pending pattern length update from a slot load.
            let pending_len = pending_pattern_length_for_ui.swap(0, Ordering::Relaxed);
            if pending_len >= 1 && pending_len <= 64 {
                setter.set_parameter(&params_for_ui.pattern_length, pending_len as i32);
            }

            // Sync last_loaded_slot with the audio thread so the UI always
            // reflects the slot that was actually loaded (prevents divergence
            // when clicking rapidly while the audio thread is still restoring).
            let audio_slot = audio_last_loaded_slot_for_ui.load(Ordering::Relaxed);
            if audio_slot == u32::MAX {
                state.last_loaded_slot = None;
            } else if (audio_slot as usize) < SLOT_COUNT {
                state.last_loaded_slot = Some(audio_slot as usize);
            }

            ResizableWindow::new("drum-pattern-generator")
                .min_size(Vec2::new(1480.0, 900.0))
                .fixed_size(Vec2::new(1480.0, 900.0))
                .resizable(false)
                .show(egui_ctx, editor_state.as_ref(), |ui| {
                    draw_header_bar(
                        ui,
                        setter,
                        &params_for_ui,
                        state,
                        &save_pattern_request,
                        &load_pattern_request,
                        &song_mode_for_ui,
                        &song_position_for_ui,
                    );

                    let body_h = ui.available_height();
                    let body_w = ui.available_width();
                    let (body_rect, _) =
                        ui.allocate_exact_size(Vec2::new(body_w, body_h), egui::Sense::hover());
                    ui.painter().rect_filled(body_rect, 0.0, BG);

                    let right_w = 568.0;
                    let left_w = (body_rect.width() - right_w).max(0.0);
                    let left_rect =
                        egui::Rect::from_min_size(body_rect.min, Vec2::new(left_w, body_h));
                    let right_rect = egui::Rect::from_min_size(
                        egui::pos2(left_rect.right(), body_rect.top()),
                        Vec2::new(right_w, body_h),
                    );

                    ui.painter().vline(
                        left_rect.right(),
                        body_rect.y_range(),
                        egui::Stroke::new(1.0, LINE),
                    );
                    ui.painter().rect_filled(right_rect, 0.0, PANEL);

                    ui.allocate_new_ui(
                        egui::UiBuilder::new()
                            .max_rect(left_rect.shrink2(Vec2::new(14.0, 14.0)))
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                        |ui| {
                            ui.set_clip_rect(left_rect.shrink2(Vec2::new(1.0, 0.0)));
                            ui.set_width(left_rect.width() - 28.0);
                            ui.set_height(left_rect.height() - 28.0);
                            ui.spacing_mut().item_spacing.y = 16.0;
                            draw_grid_v2(
                                ui,
                                setter,
                                &params_for_ui,
                                &pattern_for_ui,
                                &voice_test_triggers_for_ui,
                                &external_midi_triggers_for_ui,
                                &current_step,
                                &current_steps_for_ui,
                                &sound_settings_for_ui,
                                &plock_for_ui,
                                state,
                            );
                            draw_pattern_bank(
                                ui,
                                state,
                                &params_for_ui,
                                &pattern_for_ui,
                                &save_pattern_request,
                                &load_pattern_request,
                                &clear_plocks_request_for_ui,
                            );
                            draw_bottom_panel(
                                ui,
                                setter,
                                &params_for_ui,
                                &pattern_for_ui,
                                state,
                                &song_mode_for_ui,
                                &song_position_for_ui,
                            );
                        },
                    );

                    ui.allocate_new_ui(
                        egui::UiBuilder::new()
                            .max_rect(right_rect)
                            .layout(egui::Layout::top_down(egui::Align::Min)),
                        |ui| {
                            ui.painter().rect_filled(ui.max_rect(), 0.0, PANEL);
                            ui.set_width(right_w);
                            ui.set_height(body_h);
                            draw_sound_panel(
                                ui,
                                &sound_settings_for_ui,
                                &params_for_ui,
                                setter,
                                state,
                            );
                        },
                    );

                    // Custom plock popup to avoid egui context_menu chrome.
                    draw_plock_popup(
                        egui_ctx,
                        setter,
                        &params_for_ui,
                        &pattern_for_ui,
                        &sound_settings_for_ui,
                        &plock_for_ui,
                        state,
                    );
                });
        },
    )
}

// ---------------------------------------------------------------------------------------------------------------
// Header bar: Brand + Play + BPM + Sliders + Toggles
// ---------------------------------------------------------------------------------------------------------------
/// Thin-pill header slider bound to a nih-plug parameter (Master / Swing).
/// Layout: left word-label · flexible 6px track (BLUE fill, hover knob) · right mono value.
fn header_param_slider<P: Param>(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &P,
    total_w: f32,
    label: &str,
    show_value: bool,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, CTL_HEIGHT), egui::Sense::hover());
    let cy = rect.center().y;
    let norm = param.unmodulated_normalized_value();

    let painter = ui.painter_at(rect);
    let track_left = if label.is_empty() {
        rect.left()
    } else {
        let label_rect = painter.text(
            egui::pos2(rect.left(), cy),
            egui::Align2::LEFT_CENTER,
            label,
            f_sans_med(11.5),
            INK2,
        );
        label_rect.right().max(rect.left() + 34.0) + 8.0
    };
    let track_right = if show_value {
        let valstr = param.normalized_value_to_string(norm, true);
        let val_rect = painter.text(
            egui::pos2(rect.right(), cy),
            egui::Align2::RIGHT_CENTER,
            &valstr,
            f_mono(11.0),
            INK,
        );
        (val_rect.left() - 8.0).max(track_left + 12.0)
    } else {
        rect.right()
    };
    // Reserve the Ø11 knob radius at both ends so it isn't clipped at the extremes.
    let knob_r = 6.0;
    let track_left = track_left.max(rect.left() + knob_r);
    let track_right = track_right
        .min(rect.right() - knob_r)
        .max(track_left + 12.0);
    let track = egui::Rect::from_min_max(
        egui::pos2(track_left, cy - 3.0),
        egui::pos2(track_right, cy + 3.0),
    );

    let resp = ui.interact(
        track.expand2(Vec2::new(0.0, 8.0)),
        ui.make_persistent_id(("hslider", label)),
        egui::Sense::click_and_drag(),
    );
    let mut frac = norm;
    let frac_at = |x: f32| ((x - track.left()) / track.width().max(1.0)).clamp(0.0, 1.0);
    if resp.drag_started() {
        setter.begin_set_parameter(param);
    }
    if (resp.dragged() || resp.drag_started()) && resp.interact_pointer_pos().is_some() {
        frac = frac_at(resp.interact_pointer_pos().unwrap().x);
        setter.set_parameter_normalized(param, frac);
    }
    if resp.drag_stopped() {
        setter.end_set_parameter(param);
    }
    if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            frac = frac_at(p.x);
            setter.begin_set_parameter(param);
            setter.set_parameter_normalized(param, frac);
            setter.end_set_parameter(param);
        }
    }

    painter.rect_filled(track, 3.0, PANEL2);
    let fill_w = (track.width() * frac).max(0.0);
    if fill_w > 0.5 {
        painter.rect_filled(
            egui::Rect::from_min_max(track.min, egui::pos2(track.left() + fill_w, track.max.y)),
            3.0,
            BLUE,
        );
    }
    if resp.hovered() || resp.dragged() {
        painter.circle_filled(
            egui::pos2(track.left() + fill_w, cy),
            5.5,
            Color32::from_rgb(0xee, 0xf2, 0xf8),
        );
    }
}

/// A 1px vertical separator (height 22) with 14pt horizontal padding on each side.
fn header_vbar(ui: &mut egui::Ui) {
    ui.add_space(14.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, LINE);
    ui.add_space(14.0);
}

fn draw_header_bar(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    _state: &mut EditorUIState,
    _save_pattern_request: &Arc<AtomicU32>,
    _load_pattern_request: &Arc<AtomicU32>,
    _song_mode: &Arc<AtomicBool>,
    _song_position: &Arc<AtomicU32>,
) {
    let available = ui.available_size_before_wrap();
    let header_height = HEADER_H;
    let (rect, _) = ui.allocate_exact_size(
        egui::Vec2::new(available.x, header_height),
        egui::Sense::hover(),
    );

    // Fond PANEL + bordure basse LINE
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, PANEL);
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0, LINE),
    );

    // Contenu avec padding horizontal
    let content_rect = rect.shrink2(egui::Vec2::new(14.0, 0.0));
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(content_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.horizontal_centered(|ui| {
                ui.set_height(content_rect.height());
                ui.spacing_mut().item_spacing.x = 0.0;

                // Brand
                ui.label(
                    RichText::new("FLASH DRUM")
                        .font(f_sans_bold(15.0))
                        .color(Color32::WHITE),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("v{} · {}", env!("CARGO_PKG_VERSION"), BUILD_ID))
                        .font(f_mono(9.5))
                        .color(INK3),
                );

                header_vbar(ui);

                // Master / Swing sliders + Groove select
                header_param_slider(ui, setter, &params.master_volume, 172.0, "Master", true);
                header_vbar(ui);
                header_param_slider(ui, setter, &params.swing, 172.0, "Swing", true);
                ui.add_space(8.0);
                enum_combo(ui, setter, &params.groove_type, "");

                header_vbar(ui);

                // Sequencer source: Internal sequencer vs external MIDI from the host.
                ui.label(RichText::new("Seq").font(f_sans_sb(10.5)).color(INK3));
                ui.add_space(8.0);
                let internal = params.use_internal_sequencer.value();
                let sel =
                    led_segmented(ui, &["Internal", "Ext MIDI"], if internal { 0 } else { 1 });
                let want_internal = sel == 0;
                if want_internal != internal {
                    setter.begin_set_parameter(&params.use_internal_sequencer);
                    setter.set_parameter(&params.use_internal_sequencer, want_internal);
                    setter.end_set_parameter(&params.use_internal_sequencer);
                }

                header_vbar(ui);

                // Toggles (LED pills)
                toggle_led_param(ui, setter, &params.hihat_chokes_oh, "Choke");
                ui.add_space(6.0);
                toggle_led_param(ui, setter, &params.auto_edit, "Auto-Edit");
            });
        },
    );
}

// ---------------------------------------------------------------------------------------------------------------
// Pattern bank bar: Save/Load with P1-P8 slots
// ---------------------------------------------------------------------------------------------------------------
fn draw_pattern_bank(
    ui: &mut egui::Ui,
    state: &mut EditorUIState,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    save_pattern_request: &Arc<AtomicU32>,
    load_pattern_request: &Arc<AtomicU32>,
    clear_plocks_request: &Arc<AtomicBool>,
) {
    // Count active plocks for debug display
    let mut sound_plock_count = 0usize;
    let mut seq_plock_count = 0usize;
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        for step in 0..64usize {
            if params.plock_state.state.masks.is_active(inst, step) {
                sound_plock_count += 1;
            }
            if params.seq_plock_state.state.is_active(inst, step) {
                seq_plock_count += 1;
            }
        }
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("Patterns").strong().size(12.0).color(INK));
        ui.label(
            RichText::new(format!("[P:{} S:{}]", sound_plock_count, seq_plock_count))
                .size(9.0)
                .monospace()
                .color(FAINT),
        );
        ui.add_space(8.0);

        // MIDI export chips (left side, always visible)
        if chip_button(ui, "Export", false, BLUE, egui::Sense::click()).clicked() {
            let bpm = params.bpm.value();
            let pattern_length = params.pattern_length.value() as usize;
            let swing = params.swing.value();
            let groove_type = params.groove_type.value();
            match export_midi_to_documents(
                pattern,
                &params.track_layout.state,
                bpm,
                pattern_length,
                swing,
                groove_type,
            ) {
                Ok(path) => {
                    nih_log!("MIDI exported to: {}", path.display());
                    state.last_midi_export_path = Some(path.display().to_string());
                    state.last_midi_export_error = None;
                }
                Err(e) => {
                    nih_log!("MIDI export failed: {}", e);
                    state.last_midi_export_path = None;
                    state.last_midi_export_error = Some(e.to_string());
                }
            }
        }
        let drag_response = chip_button(ui, "Drag", false, BLUE, egui::Sense::click())
            .on_hover_text("Drag the current pattern into your DAW");
        if drag_response.clicked() {
            let bpm = params.bpm.value();
            let pattern_length = params.pattern_length.value() as usize;
            let swing = params.swing.value();
            let groove_type = params.groove_type.value();
            match export_midi_to_documents(
                pattern,
                &params.track_layout.state,
                bpm,
                pattern_length,
                swing,
                groove_type,
            )
            .and_then(|path| start_external_midi_drag(&path).map(|_| path))
            {
                Ok(path) => {
                    nih_log!("MIDI drag helper started from: {}", path.display());
                    state.last_midi_export_path = Some(path.display().to_string());
                    state.last_midi_export_error = None;
                }
                Err(e) => {
                    nih_log!("MIDI drag failed: {}", e);
                    state.last_midi_export_path = None;
                    state.last_midi_export_error = Some(e.to_string());
                }
            }
        }

        ui.add_space(8.0);

        // Save button (blinks when save mode is active)
        let is_save_mode = state.save_mode_active;
        let time = ui.ctx().input(|i| i.time);
        let blink = if is_save_mode {
            ((time * 4.0).sin() + 1.0) / 2.0 // 0..1 oscillation
        } else {
            0.0
        };
        let save_fill = if is_save_mode {
            Color32::from_rgb(
                (74.0 + blink * 80.0) as u8,
                (158.0 + blink * 40.0) as u8,
                255,
            )
        } else {
            PANEL2
        };
        let save_btn = egui::Button::new(RichText::new("Save").size(10.0).strong().monospace())
            .min_size(Vec2::new(44.0, 22.0))
            .fill(save_fill)
            .stroke(egui::Stroke::new(
                1.5,
                if is_save_mode { BLUE } else { LINE2 },
            ))
            .corner_radius(5.0);
        let save_response = ui.add(save_btn);
        let save_response = save_response.on_hover_text(
            RichText::new(if is_save_mode {
                "Click a slot (P1-P8) to save the current pattern there"
            } else {
                "Activate save mode, then click a slot to store the current pattern"
            })
            .size(11.0)
            .monospace(),
        );
        if save_response.clicked() {
            state.save_mode_active = !state.save_mode_active;
            state.clear_confirm_mode = false;
        }

        ui.add_space(8.0);

        // Determine if current pattern is dirty compared to last_loaded_slot
        let is_dirty = state.last_loaded_slot.map_or(false, |slot_idx| {
            if let Ok(bank) = params.pattern_bank.bank.lock() {
                let slot = &bank.slots[slot_idx];
                if !slot.occupied {
                    return false;
                }
                // Compare step masks
                let current_masks = pattern.step_masks();
                if slot.step_masks != current_masks {
                    return true;
                }
                // Compare pattern length
                let current_len = params.pattern_length.value() as u8;
                if slot.pattern_length != current_len {
                    return true;
                }
                false
            } else {
                false
            }
        });

        // P1-P8 slots
        for i in 0..8 {
            let occupied = params
                .pattern_bank
                .bank
                .lock()
                .map(|b| b.slots[i].occupied)
                .unwrap_or(false);
            let is_loaded = state.last_loaded_slot == Some(i);
            let show_star = is_dirty && is_loaded;
            let text = if show_star {
                format!("P{}*", i + 1)
            } else {
                format!("P{}", i + 1)
            };

            let btn_size = Vec2::new(30.0, 22.0);
            let fill = if is_loaded {
                P_ACTIVE
            } else if occupied {
                PANEL2
            } else {
                BG // much darker for empty slot
            };
            let stroke_color = if is_loaded {
                GREEN // green ring for loaded
            } else if occupied {
                LINE2
            } else {
                LINE // dimmer border for empty slot
            };

            let response = ui
                .allocate_ui_with_layout(
                    btn_size,
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        let (rect, response) =
                            ui.allocate_exact_size(btn_size, egui::Sense::click());
                        let visuals = ui.style().interact(&response);
                        let rect = rect.expand(visuals.expansion);
                        let corner_radius = 5.0;
                        ui.painter().rect_filled(rect, corner_radius, fill);
                        ui.painter().rect_stroke(
                            rect,
                            corner_radius,
                            egui::Stroke::new(2.0, stroke_color),
                            egui::StrokeKind::Outside,
                        );

                        let label_color = if is_loaded { GREEN } else { INK };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &text,
                            egui::FontId::monospace(10.0),
                            label_color,
                        );
                        response
                    },
                )
                .inner;

            let tooltip = if state.save_mode_active {
                format!("Save current pattern to P{}", i + 1)
            } else if occupied {
                if is_loaded {
                    format!(
                        "P{}: current pattern (click to reload)\n* = unsaved changes",
                        i + 1
                    )
                } else {
                    format!("P{}: saved pattern (click to load)", i + 1)
                }
            } else {
                format!("P{}: empty", i + 1)
            };
            let response = response.on_hover_text(RichText::new(tooltip).size(11.0).monospace());

            if response.clicked_by(egui::PointerButton::Primary) {
                state.clear_confirm_mode = false;
                if state.save_mode_active {
                    save_pattern_request
                        .store((i + 1) as u32, std::sync::atomic::Ordering::Relaxed);
                    state.save_mode_active = false;
                    state.last_loaded_slot = Some(i);
                } else if occupied {
                    load_pattern_request
                        .store((i + 1) as u32, std::sync::atomic::Ordering::Relaxed);
                    state.last_loaded_slot = Some(i);
                }
            }
            if response.secondary_clicked() {
                if occupied {
                    // Copy slot to clipboard
                    if let Ok(bank) = params.pattern_bank.bank.lock() {
                        state.pattern_clipboard = Some(bank.slots[i].clone());
                    }
                } else if let Some(ref clipboard) = state.pattern_clipboard {
                    // Paste clipboard into empty slot
                    if let Ok(mut bank_mut) = params.pattern_bank.bank.lock() {
                        bank_mut.slots[i] = clipboard.clone();
                    }
                }
            }
        }

        ui.add_space(8.0);

        // Clear button: wipes all sound + sequencer plocks (two-step confirmation)
        let is_clear_confirm = state.clear_confirm_mode;
        let clear_blink = if is_clear_confirm {
            ((time * 4.0).sin() + 1.0) / 2.0 // 0..1 oscillation
        } else {
            0.0
        };
        let clear_fill = if is_clear_confirm {
            Color32::from_rgb(
                (200.0 + clear_blink * 55.0) as u8,
                (60.0 + clear_blink * 40.0) as u8,
                (60.0 + clear_blink * 40.0) as u8,
            )
        } else {
            PANEL2
        };
        let clear_btn = egui::Button::new(
            RichText::new(if is_clear_confirm { "Sure?" } else { "Clr" })
                .size(10.0)
                .strong()
                .monospace(),
        )
        .min_size(Vec2::new(44.0, 26.0))
        .fill(clear_fill)
        .stroke(egui::Stroke::new(
            1.5,
            if is_clear_confirm {
                Color32::from_rgb(255, 120, 120)
            } else {
                LINE2
            },
        ))
        .corner_radius(5.0);
        let clear_response = ui.add(clear_btn);
        let clear_response = clear_response.on_hover_text(
            RichText::new(if is_clear_confirm {
                "Click again to confirm clearing the current pattern"
            } else {
                "Clear all steps and plocks from the current pattern"
            })
            .size(11.0)
            .monospace(),
        );
        if clear_response.clicked() {
            if is_clear_confirm {
                // Confirmed: clear grid + plocks + fusions
                load_pattern_for_ui(pattern, &crate::sequencer::pattern::Pattern::empty());
                params.plock_state.state.clear_all();
                params.seq_plock_state.state.clear_all();
                // Clear all fusions
                for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
                    pattern.store_fusions(inst, &[]);
                }
                state.last_loaded_slot = None;
                state.clear_confirm_mode = false;
            } else {
                // Enter confirmation mode, cancel save mode if active
                state.clear_confirm_mode = true;
                state.save_mode_active = false;
            }
        }

        ui.add_space(8.0);

        if let Some(path) = &state.last_midi_export_path {
            if ui.button("Copy Path").clicked() {
                ui.ctx().copy_text(path.clone());
            }
            ui.label(RichText::new("Exported").size(10.0));
        } else if state.last_midi_export_error.is_some() {
            ui.label(
                RichText::new("Export failed")
                    .size(10.0)
                    .color(Color32::from_rgb(248, 113, 113)),
            );
        }
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Bottom panel: Generator | Song (shared panel with toggle)
// ---------------------------------------------------------------------------------------------------------------
fn draw_bottom_panel(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
    _song_mode: &Arc<AtomicBool>,
    song_position: &Arc<AtomicU32>,
) {
    let panel_w = ui.available_width();
    let panel_h = 210.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(panel_w, panel_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, RADIUS_PANEL, PANEL);
    painter.rect_stroke(
        rect,
        RADIUS_PANEL,
        egui::Stroke::new(1.0, LINE),
        egui::StrokeKind::Inside,
    );
    painter.hline(
        rect.x_range(),
        rect.top() + 42.0,
        egui::Stroke::new(1.0, LINE),
    );

    let header_rect = egui::Rect::from_min_size(rect.min, Vec2::new(panel_w, 42.0));
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(header_rect.shrink2(Vec2::new(12.0, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_clip_rect(header_rect);
            ui.horizontal(|ui| {
                ui.set_height(42.0);
                ui.spacing_mut().item_spacing.x = 0.0;
                // Generator | Song segmented tabs: view only, no longer toggle song mode.
                let selected = state.bottom_panel_tab.min(1);
                let new_selected = generator_song_segmented(ui, selected);
                if new_selected != selected {
                    state.bottom_panel_tab = new_selected;
                }

                ui.add_space(12.0);

                // Meta text
                let meta = if state.bottom_panel_tab == 1 {
                    if let Ok(bank) = params.pattern_bank.bank.lock() {
                        let blocks = (bank.song.length as usize).min(SONG_BLOCKS);
                        let total_reps = bank.song.steps[..blocks]
                            .iter()
                            .filter(|&&s| s >= 0)
                            .count();
                        format!("{} blocks · {} patterns", blocks, total_reps)
                    } else {
                        "Song chain".to_string()
                    }
                } else {
                    format!(
                        "{} · {} -> {}",
                        GeneratorType::variants()[params.generator_type.value().to_index()],
                        Style::variants()[params.style_primary.value().to_index()],
                        Style::variants()[params.style_secondary.value().to_index()]
                    )
                };
                ui.label(RichText::new(meta).monospace().size(10.5).color(INK3));
            });
        },
    );

    let body_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 12.0, rect.top() + 44.0),
        egui::pos2(rect.right() - 12.0, rect.bottom() - 8.0),
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(body_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.set_clip_rect(body_rect);
            ui.set_width(body_rect.width());
            ui.set_height(body_rect.height());
            if state.bottom_panel_tab == 1 {
                draw_song_editor(ui, setter, params, state, song_position);
            } else {
                draw_generator_panel_content(ui, setter, params, pattern, state);
            }
        },
    );
}

// ---------------------------------------------------------------------------------------------------------------
// Generator panel content (preset chips + generator controls + GENERATE button)
// ---------------------------------------------------------------------------------------------------------------
fn draw_generator_panel_content(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
) {
    ui.vertical(|ui| {
        ui.add_space(6.0);
        ui.spacing_mut().item_spacing.y = 12.0;
        draw_generator_bar(ui, setter, params, pattern, state);
        draw_preset_bar(ui, pattern, params, setter, state);
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Presets / Random
// ---------------------------------------------------------------------------------------------------------------
fn draw_preset_bar(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    state: &mut EditorUIState,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 7.0;
        ui.set_height(CTL_HEIGHT);
        genrow_label(ui, "Presets", 62.0);
        let pattern_length = params.pattern_length.value() as usize;
        if compact_chip(ui, "Rock", false).clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::rock_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        if compact_chip(ui, "Funk", false).clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::funk_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        if compact_chip(ui, "Disco", false).clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::disco_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        ui.add_space(8.0);
        if chip_button(ui, "⟳ Random", true, PL_LINK, egui::Sense::click()).clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::random_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Generator parameters
// ---------------------------------------------------------------------------------------------------------------
fn draw_generator_bar(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
) {
    const GEN_TYPE_W: f32 = 116.0;
    const STYLE_W: f32 = 70.0;
    const GEN_BTN_W: f32 = 104.0;

    // Single aligned row: algorithm · style A → B · Mix/Dens/Var · GENERATE (right).
    ui.horizontal(|ui| {
        ui.set_height(CTL_HEIGHT);
        ui.spacing_mut().item_spacing.x = 0.0;

        // Generator algorithm (Probabilistic / Euclidean / Markov / …)
        enum_combo_compact(ui, setter, &params.generator_type, "gen_type", GEN_TYPE_W);

        // Style morph A → B
        ui.add_space(16.0);
        genrow_label(ui, "A", 12.0);
        ui.add_space(5.0);
        enum_combo_compact(ui, setter, &params.style_primary, "style_a", STYLE_W);
        ui.add_space(12.0);
        genrow_label(ui, "B", 12.0);
        ui.add_space(5.0);
        enum_combo_compact(ui, setter, &params.style_secondary, "style_b", STYLE_W);

        // Amounts (design-system pill sliders)
        ui.add_space(18.0);
        const SLIDER_TOTAL_W: f32 = 110.0;
        header_param_slider(
            ui,
            setter,
            &params.style_mix,
            SLIDER_TOTAL_W,
            "Mix A/B",
            false,
        );
        ui.add_space(10.0);
        header_param_slider(
            ui,
            setter,
            &params.gen_density,
            SLIDER_TOTAL_W,
            "Density",
            false,
        );
        ui.add_space(10.0);
        header_param_slider(
            ui,
            setter,
            &params.gen_variation,
            SLIDER_TOTAL_W,
            "Variation",
            false,
        );

        // GENERATE, pushed to the right edge
        let space = (ui.available_width() - GEN_BTN_W).max(10.0);
        ui.add_space(space);
        let gen_btn_response = ui.add_sized(
            Vec2::new(GEN_BTN_W, CTL_HEIGHT),
            egui::Button::new(
                RichText::new("GENERATE")
                    .font(f_sans_sb(11.0))
                    .color(Color32::WHITE),
            )
            .fill(BLUE)
            .stroke(egui::Stroke::new(1.0, BLUE))
            .corner_radius(6.0),
        );

        if gen_btn_response.clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0);
            let gen_params = generator::GeneratorParams {
                generator_type: params.generator_type.value(),
                style_primary: params.style_primary.value(),
                style_secondary: params.style_secondary.value(),
                style_mix: params.style_mix.value(),
                density: params.gen_density.value(),
                variation: params.gen_variation.value(),
                seed,
            };
            let generated = generator::generate(&gen_params, params.track_layout.state.as_ref());
            let pattern_length = params.pattern_length.value() as usize;
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &generated, pattern_length);
            state.last_loaded_slot = None;
        }
    });
}
fn draw_song_editor(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    state: &mut EditorUIState,
    song_position: &Arc<AtomicU32>,
) {
    let current_song_pos = song_position.load(Ordering::Relaxed) as usize;
    let is_song_active = params.song_mode.value();

    let mut bank = match params.pattern_bank.bank.lock() {
        Ok(b) => b,
        Err(_) => return,
    };

    // Always use 16 song blocks and always loop.
    bank.song.length = SONG_BLOCKS as u8;
    bank.song.loop_enabled = true;

    let selected = state.song_selected_step.min(SONG_BLOCKS - 1);

    // Header: Song Mode checkbox, Clear All
    ui.horizontal(|ui| {
        ui.set_height(26.0);
        ui.spacing_mut().item_spacing.x = 6.0;

        let mut song_enabled = params.song_mode.value();
        if ui.checkbox(&mut song_enabled, "Song Mode").changed() {
            setter.set_parameter(&params.song_mode, song_enabled);
        }

        ui.add_space(8.0);
        if state.song_clear_confirm {
            let btn = egui::Button::new(RichText::new("Confirm?").size(10.0).color(Color32::WHITE))
                .min_size(Vec2::new(70.0, 20.0))
                .fill(Color32::from_rgb(180, 60, 60))
                .stroke(egui::Stroke::new(1.0, LINE2))
                .corner_radius(5.0);
            if ui.add(btn).clicked() {
                for step in 0..SONG_BLOCKS {
                    bank.song.set_step(step, -1);
                    bank.song.set_repeat(step, 1);
                }
                state.song_clear_confirm = false;
            }
        } else {
            let btn = egui::Button::new(RichText::new("Clear All").size(10.0))
                .min_size(Vec2::new(70.0, 20.0))
                .fill(PANEL2)
                .stroke(egui::Stroke::new(1.0, LINE2))
                .corner_radius(5.0);
            if ui.add(btn).clicked() {
                state.song_clear_confirm = true;
            }
        }
    });

    ui.add_space(4.0);

    // Step grid: 1 row of 16 editable blocks (pattern on top, repeat on bottom).
    let body_w = ui.available_width();
    let cell_h = 64.0;
    let cell_w = ((body_w - 2.0 * (SONG_BLOCKS as f32 - 1.0)) / SONG_BLOCKS as f32).max(18.0);
    let steps_per_row = SONG_BLOCKS;

    ui.horizontal(|ui| {
        ui.set_height(cell_h);
        ui.spacing_mut().item_spacing.x = 2.0;
        for step_idx in 0..steps_per_row {
            let is_current = step_idx == current_song_pos && is_song_active;
            let is_selected = step_idx == selected;
            let slot = bank.song.steps[step_idx];
            let occupied =
                slot >= 0 && (slot as usize) < SLOT_COUNT && bank.slots[slot as usize].occupied;

            let fill = if is_current {
                BLUE
            } else if occupied {
                PANEL2
            } else {
                Color32::from_rgb(18, 18, 24)
            };
            let stroke_color = if is_selected {
                BLUE
            } else if is_current {
                BLUE
            } else {
                LINE2
            };

            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(cell_w, cell_h), egui::Sense::click());
            ui.painter().rect_filled(rect, 3.0, fill);
            ui.painter().rect_stroke(
                rect,
                3.0,
                egui::Stroke::new(1.0, stroke_color),
                egui::StrokeKind::Inside,
            );

            // Top half: pattern selector.
            let inner = rect.shrink(2.0);
            let top_rect = egui::Rect::from_min_size(
                inner.min,
                Vec2::new(inner.width(), inner.height() * 0.5),
            );
            ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(top_rect)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
                |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        let mut slot = bank.song.steps[step_idx];
                        let selected_text = if slot < 0 {
                            "--".to_string()
                        } else {
                            format!("P{}", slot + 1)
                        };
                        egui::ComboBox::from_id_salt(format!("song_pattern_select_{}", step_idx))
                            .selected_text(selected_text)
                            .width(ui.available_width().max(20.0))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut slot, -1, "--");
                                for i in 0..SLOT_COUNT {
                                    if bank.slots[i].occupied {
                                        let text = format!("P{}", i + 1);
                                        ui.selectable_value(&mut slot, i as i8, text);
                                    }
                                }
                            });
                        if slot != bank.song.steps[step_idx] {
                            bank.song.set_step(step_idx, slot);
                        }
                    });
                },
            );

            // Bottom half: repeat editor.
            let bottom_rect = egui::Rect::from_min_size(
                egui::pos2(inner.left(), inner.top() + inner.height() * 0.5),
                Vec2::new(inner.width(), inner.height() * 0.5),
            );
            ui.allocate_new_ui(
                egui::UiBuilder::new()
                    .max_rect(bottom_rect)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
                |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        let mut repeat = bank.song.repeats[step_idx];
                        ui.add_sized(
                            Vec2::new(ui.available_width(), ui.available_height()),
                            egui::DragValue::new(&mut repeat)
                                .range(1..=64)
                                .speed(1.0)
                                .fixed_decimals(0)
                                .custom_formatter(|n, _| {
                                    if n <= 1.0 {
                                        "x1".to_string()
                                    } else {
                                        format!("x{}", n as i64)
                                    }
                                }),
                        );
                        if repeat != bank.song.repeats[step_idx] {
                            bank.song.set_repeat(step_idx, repeat);
                        }
                    });
                },
            );

            if response.clicked() {
                state.song_selected_step = step_idx;
            }
            response.context_menu(|ui| {
                if ui.button("Copy").clicked() {
                    state.song_clipboard =
                        Some((bank.song.steps[step_idx], bank.song.repeats[step_idx]));
                    ui.close_menu();
                }
                if ui
                    .add_enabled(state.song_clipboard.is_some(), egui::Button::new("Paste"))
                    .clicked()
                {
                    if let Some((slot, repeat)) = state.song_clipboard {
                        bank.song.set_step(step_idx, slot);
                        bank.song.set_repeat(step_idx, repeat);
                    }
                    ui.close_menu();
                }
                if ui.button("Duplicate").clicked() {
                    let next = step_idx + 1;
                    if next < SONG_BLOCKS {
                        let slot = bank.song.steps[step_idx];
                        let repeat = bank.song.repeats[step_idx];
                        bank.song.set_step(next, slot);
                        bank.song.set_repeat(next, repeat);
                    }
                    ui.close_menu();
                }
                if ui.button("Clear").clicked() {
                    bank.song.set_step(step_idx, -1);
                    bank.song.set_repeat(step_idx, 1);
                    ui.close_menu();
                }
            });
        }
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Pattern grid with per-row Hum/Push/Len
// ---------------------------------------------------------------------------------------------------------------
fn draw_grid_v2(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    voice_test_triggers: &[AtomicBool; crate::track::MAX_TRACKS],
    external_midi_triggers: &[AtomicBool; crate::track::MAX_TRACKS],
    current_step: &AtomicU32,
    current_steps: &[AtomicU32; crate::track::MAX_TRACKS],
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
) {
    let master_length = params.pattern_length.value().clamp(1, 64) as usize;
    let play_step = current_step.load(Ordering::Relaxed) as usize;
    let play_page = play_step / 16;

    draw_page_bar_v2(
        ui,
        setter,
        params,
        pattern,
        sound_settings,
        plock,
        state,
        play_page,
        master_length,
    );

    if state.follow_mode && play_page < 4 {
        state.current_page = play_page;
    }

    let page_offset = state.current_page * 16;
    let fusion_mode_active = fusion_modifier_pressed(ui);
    if !fusion_mode_active {
        for selection_start in state.fusion_selection_start.iter_mut() {
            *selection_start = None;
        }
    }

    let mut fusion_editing_started_this_frame = false;

    egui::Frame::new()
        .fill(PANEL)
        .stroke(egui::Stroke::new(1.0, LINE))
        .corner_radius(RADIUS_PANEL)
        .inner_margin(11.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = GAP_TIGHT;

            let row_w = ui.available_width();
            let grip_w = 14.0;
            let name_w = 34.0;
            let vol_w = 56.0;
            let mst_w = STEP_H * 3.0 + GAP_TIGHT * 2.0;
            let extra_w = 44.0;
            let gap = 7.0;
            let fixed_w = grip_w + name_w + vol_w + mst_w + extra_w * 3.0 + gap * 7.0;
            let steps_w = (row_w - fixed_w).max(320.0);
            let cell_w = ((steps_w - GAP_TIGHT * 15.0) / 16.0).floor().max(18.0);

            draw_seq_header_v2(
                ui,
                page_offset,
                play_step,
                grip_w,
                name_w,
                vol_w,
                mst_w,
                extra_w,
                gap,
                cell_w,
            );

            let mixer_rows = mixer_rows(params);
            let hums: [&FloatParam; crate::track::MAX_TRACKS] =
                std::array::from_fn(|i| params.humanizes()[i]);
            let pushes: [&FloatParam; crate::track::MAX_TRACKS] =
                std::array::from_fn(|i| params.pushes()[i]);
            let lengths: [&IntParam; crate::track::MAX_TRACKS] =
                std::array::from_fn(|i| params.lengths()[i]);
            state.selected_track_slot = state.selected_track_slot.min(crate::track::MAX_TRACKS - 1);
            let mut lane_row_rects: [Option<egui::Rect>; crate::track::MAX_TRACKS] =
                [None; crate::track::MAX_TRACKS];

            // Always render the full 14 rows (active lanes + styled empty lanes)
            // so the grid height is constant and the panels below never shift.
            for slot_idx in 0..crate::track::MAX_TRACKS {
                let Some(inst) = voice_idx_for_slot(params, slot_idx) else {
                    // Inactive slot: the +N chip opens the instrument picker
                    // for this specific slot.
                    let (row_response, add_pos) = draw_empty_slot_lane_v2(
                        ui,
                        setter,
                        params,
                        slot_idx,
                        page_offset,
                        grip_w,
                        name_w,
                        vol_w,
                        mst_w,
                        extra_w,
                        gap,
                        cell_w,
                        state,
                        sound_settings,
                        pattern,
                        plock,
                    );
                    lane_row_rects[slot_idx] = Some(row_response.rect);
                    if let Some(pos) = add_pos {
                        state.add_module_popup = Some(AddModulePopup {
                            slot: slot_idx,
                            screen_pos: pos,
                        });
                    }
                    continue;
                };
                let row = &mixer_rows[slot_idx];
                let fusions = pattern.load_fusions(slot_idx);
                let lane_length = effective_lane_length_for_ui(params, slot_idx, master_length);
                let lane_play_step = current_steps[slot_idx].load(Ordering::Relaxed) as usize;
                let row_response = draw_legacy_slot_lane_v2(
                    ui,
                    setter,
                    params,
                    pattern,
                    voice_test_triggers,
                    external_midi_triggers,
                    sound_settings,
                    plock,
                    state,
                    slot_idx,
                    inst,
                    row,
                    &fusions,
                    hums[slot_idx],
                    pushes[slot_idx],
                    lengths[slot_idx],
                    page_offset,
                    lane_play_step,
                    master_length,
                    lane_length,
                    fusion_mode_active,
                    grip_w,
                    name_w,
                    vol_w,
                    extra_w,
                    gap,
                    cell_w,
                    &mut fusion_editing_started_this_frame,
                );
                lane_row_rects[slot_idx] = Some(row_response.rect);
            }

            if state.lane_drag_source.is_some() {
                if let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) {
                    if let Some(gap) = compute_reorder_gap(&lane_row_rects, pointer_pos) {
                        draw_lane_reorder_indicator(ui, &lane_row_rects, gap);
                    }
                }
            }

            handle_lane_reorder_drop(
                ui,
                setter,
                params,
                pattern,
                sound_settings,
                plock,
                state,
                &lane_row_rects,
            );
        });

    let mut fusion_edit_box_rect = None;
    ui.horizontal(|ui| {
        ui.set_height(28.0);
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.label(
            RichText::new("P-Lock Mode")
                .font(f_sans_sb(10.5))
                .color(INK3),
        );
        let selected = if state.sequencer_mode { 1 } else { 0 };
        let new_selected = p_lock_mode_segmented(ui, selected);
        if new_selected != selected {
            state.sequencer_mode = new_selected == 1;
        }
        ui.label(
            RichText::new("Right-click a step to edit its p-lock")
                .size(10.5)
                .color(INK3),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            fusion_edit_box_rect = Some(draw_fusion_edit_box(
                ui,
                pattern,
                params,
                sound_settings,
                state,
                fusion_mode_active,
            ));
        });
    });

    if !fusion_editing_started_this_frame {
        close_fusion_editing_on_outside_click(ui, pattern, state, None, fusion_edit_box_rect);
    }

    draw_page_popup_if_any(ui, setter, pattern, params, plock, state);
    draw_add_module_popup_if_any(ui, params, sound_settings, state);
}

#[allow(clippy::too_many_arguments)]
fn draw_legacy_slot_lane_v2(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    voice_test_triggers: &[AtomicBool; crate::track::MAX_TRACKS],
    external_midi_triggers: &[AtomicBool; crate::track::MAX_TRACKS],
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
    slot_idx: usize,
    voice_idx: usize,
    row: &MixerRow<'_>,
    fusions: &[FusedGroup],
    hum_param: &FloatParam,
    push_param: &FloatParam,
    length_param: &IntParam,
    page_offset: usize,
    play_step: usize,
    master_length: usize,
    lane_length: usize,
    fusion_mode_active: bool,
    grip_w: f32,
    name_w: f32,
    vol_w: f32,
    extra_w: f32,
    gap: f32,
    cell_w: f32,
    fusion_editing_started_this_frame: &mut bool,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.set_height(LANE_H);

        let grip_response = draw_seq_grip_v2(ui, grip_w, LANE_H)
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text("Drag to reorder lane");
        if grip_response.is_pointer_button_down_on() || grip_response.drag_started() {
            state.lane_drag_source = Some(slot_idx);
            select_legacy_track(state, slot_idx);
        }
        if state.lane_drag_source == Some(slot_idx) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        let selected = state.selected_track_slot == slot_idx;
        let name_response = draw_lane_name_v2(
            ui,
            name_w,
            selected,
            crate::instrument_registry::INSTRUMENTS[voice_idx].label,
        )
        .on_hover_text(crate::instrument_registry::INSTRUMENTS[voice_idx].full_name);
        if name_response.clicked() {
            select_legacy_track(state, slot_idx);
        }

        name_response.context_menu(|ui| {
            if ui.button("Copy Lane").clicked() {
                state.copy_lane(params, slot_idx, sound_settings, pattern, plock);
                state.lane_clear_grid_confirm = None;
                ui.close_menu();
            }
            if ui
                .add_enabled(
                    state.lane_clipboard.is_some(),
                    egui::Button::new("Paste Lane"),
                )
                .clicked()
            {
                if state.paste_lane(setter, params, slot_idx, sound_settings, pattern, plock) {
                    // Flash visual feedback
                    state.slot_flash_until[slot_idx] = ui.ctx().input(|i| i.time) + 0.5;
                }
                state.lane_clear_grid_confirm = None;
                ui.close_menu();
            }
            if ui
                .add_enabled(
                    state.lane_clipboard.is_some(),
                    egui::Button::new("Paste Grid"),
                )
                .clicked()
            {
                if state.paste_grid(params, slot_idx, pattern) {
                    // Flash visual feedback
                    state.slot_flash_until[slot_idx] = ui.ctx().input(|i| i.time) + 0.5;
                }
                state.lane_clear_grid_confirm = None;
                ui.close_menu();
            }
            ui.separator();
            let confirm_clear_grid = state.lane_clear_grid_confirm == Some(slot_idx);
            if ui
                .button(
                    RichText::new(if confirm_clear_grid {
                        "Confirm Clear Grid?"
                    } else {
                        "Clear Grid"
                    })
                    .font(f_sans_med(11.0))
                    .color(RED),
                )
                .on_hover_text(if confirm_clear_grid {
                    "Click again to clear this lane's steps, fusions and plocks"
                } else {
                    "Clear this lane's grid; keeps instrument, sound, routing and lane controls"
                })
                .clicked()
            {
                if confirm_clear_grid {
                    state.clear_grid(params, slot_idx, pattern, plock);
                    ui.close_menu();
                } else {
                    state.lane_clear_grid_confirm = Some(slot_idx);
                }
            }
        });

        let inst_state = &sound_settings.instruments[slot_idx];
        let mut lane_vol = f32::from_bits(inst_state.volume.load(Ordering::Relaxed));
        let lane_vol_response =
            draw_mini_value_slider(ui, &mut lane_vol, 0.0, 2.0, vol_w, BLUE, "Lane Volume");
        if lane_vol_response.clicked() || lane_vol_response.dragged() {
            select_legacy_track(state, slot_idx);
        }
        if lane_vol_response.changed() {
            inst_state
                .volume
                .store(lane_vol.to_bits(), Ordering::Relaxed);
            sound_settings.bump_version();
        }

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP_TIGHT;
            if draw_tag_param_v2(
                ui,
                setter,
                row.mute,
                "M",
                AMBER,
                Color32::from_rgb(26, 18, 6),
                "Mute",
            )
            .clicked()
                && params.auto_edit.value()
            {
                select_legacy_track(state, slot_idx);
            }
            if draw_tag_param_v2(
                ui,
                setter,
                row.solo,
                "S",
                GREEN,
                Color32::from_rgb(6, 32, 15),
                "Solo",
            )
            .clicked()
                && params.auto_edit.value()
            {
                select_legacy_track(state, slot_idx);
            }
            let now = ui.ctx().input(|i| i.time);
            if external_midi_triggers[slot_idx].swap(false, Ordering::Acquire) {
                state.slot_flash_until[slot_idx] = now + 0.10;
            }
            let is_flashing = now < state.slot_flash_until[slot_idx];
            if draw_tag_button_v2(ui, "T", AMBER, Color32::BLACK, is_flashing, "Test").clicked() {
                voice_test_triggers[slot_idx].store(true, Ordering::Release);
                if params.auto_edit.value() {
                    select_legacy_track(state, slot_idx);
                }
            }
        });

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP_TIGHT;
            for local_step in 0..16 {
                let global_step = page_offset + local_step;
                let beyond_len = global_step >= lane_length;
                let fusion_info = fusion_containing(fusions, global_step);
                let fusion_group = fusion_info.map(|(_, group)| group);
                let source_step = fusion_group
                    .map(|group| group.start_cell as usize)
                    .unwrap_or(global_step);
                let is_fusion_start = fusion_group
                    .map(|group| group.is_start(global_step))
                    .unwrap_or(false);
                let is_fusion_mid = fusion_group.is_some() && !is_fusion_start;

                let active = !beyond_len && pattern.is_active(source_step, slot_idx);
                let is_current = if let Some(group) = fusion_group {
                    group.contains(play_step) && play_step == global_step
                } else {
                    play_step == global_step
                };
                let has_sound_plock = !beyond_len && plock.masks.is_active(slot_idx, source_step);
                let field_mask = if has_sound_plock {
                    plock.field_masks.get(slot_idx, source_step)
                } else {
                    0
                };
                let all_bits = (1u64 << crate::plock::FIELD_COUNT) - 1;
                let is_snapshot = has_sound_plock && field_mask == all_bits;
                let has_seq_plock = !beyond_len
                    && params
                        .seq_plock_state
                        .state
                        .is_active(slot_idx, source_step);
                let selection_start = fusion_mode_active
                    && state.fusion_selection_start[slot_idx] == Some(global_step);

                let is_editing = state
                    .fusion_editing
                    .map(|(ei, eidx)| {
                        ei == slot_idx && fusion_info.map(|(idx, _)| idx == eidx).unwrap_or(false)
                    })
                    .unwrap_or(false);

                let (fill, stroke) = step_colors_v2(
                    ui.ctx(),
                    state.sequencer_mode,
                    local_step,
                    active,
                    has_sound_plock,
                    is_snapshot,
                    has_seq_plock,
                    is_current,
                    beyond_len,
                    selection_start,
                    is_fusion_start,
                    is_fusion_mid,
                    is_editing,
                );
                let text = if is_fusion_start {
                    fusion_group
                        .map(|g| g.step_count.to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let fusion_span = if is_fusion_start {
                    fusion_group.map(|g| g.cell_span())
                } else {
                    None
                };
                let response = draw_step_cell_v2(
                    ui,
                    Vec2::new(cell_w, STEP_H),
                    fill,
                    stroke,
                    &text,
                    !beyond_len,
                    is_current && !beyond_len,
                    is_fusion_mid,
                    fusion_span,
                    beyond_len,
                );

                if !beyond_len && response.double_clicked() {
                    if let Some((idx, _)) = fusion_info {
                        select_legacy_track(state, slot_idx);
                        state.fusion_editing = Some((slot_idx, idx));
                        state.fusion_edit_focus_request = true;
                        state.fusion_edit_steps = fusion_group.map(|g| g.step_count).unwrap_or(1);
                        *fusion_editing_started_this_frame = true;
                    }
                } else if !beyond_len && response.clicked() && fusion_mode_active {
                    select_legacy_track(state, slot_idx);
                    handle_fusion_shift_click(
                        pattern,
                        params,
                        plock,
                        slot_idx,
                        global_step,
                        master_length,
                        fusions,
                        &mut state.fusion_selection_start[slot_idx],
                    );
                } else if !beyond_len && response.clicked() {
                    // Clicking outside the currently edited fusion exits edit mode.
                    if let Some((edit_inst, edit_idx)) = state.fusion_editing {
                        let editing_this_group = fusion_info
                            .map(|(idx, _)| edit_inst == slot_idx && edit_idx == idx)
                            .unwrap_or(false);
                        if !editing_this_group {
                            finish_fusion_editing_for_ui(pattern, state);
                        }
                    }

                    if let Some(group) = fusion_group {
                        toggle_fusion_for_ui(pattern, group, slot_idx);
                    } else {
                        toggle_step_for_ui(pattern, global_step, slot_idx);
                    }
                    if params.auto_edit.value() {
                        select_legacy_track(state, slot_idx);
                    }
                    state.fusion_selection_start[slot_idx] = None;
                }

                if !beyond_len && response.secondary_clicked() {
                    select_legacy_track(state, slot_idx);
                    if let Some(pos) = response.interact_pointer_pos() {
                        state.plock_popup = Some(PlockPopup {
                            instrument: slot_idx,
                            step: source_step,
                            screen_pos: pos,
                            morph_menu: false,
                        });
                    }
                }
            }
        });

        let hum_response = draw_param_mini_slider_with_value(
            ui,
            setter,
            hum_param,
            0.0,
            1.0,
            extra_w,
            BLUE,
            "Humanize",
            |value| format!("{:>3}%", (value * 100.0).round() as i32),
        );
        if hum_response.clicked() || hum_response.dragged() || hum_response.double_clicked() {
            select_legacy_track(state, slot_idx);
        }
        let push_response = draw_param_mini_slider_with_value(
            ui,
            setter,
            push_param,
            -50.0,
            50.0,
            extra_w,
            BLUE,
            "Push/Pull",
            |value| format!("{:+.0} ms", value),
        );
        if push_response.clicked() || push_response.dragged() || push_response.double_clicked() {
            select_legacy_track(state, slot_idx);
        }
        if draw_track_length_control(ui, setter, params, length_param, slot_idx, master_length) {
            select_legacy_track(state, slot_idx);
        }
    })
    .response
}

#[allow(clippy::too_many_arguments)]
/// Returns the click position when the `+N` chip was clicked (opens the
/// instrument picker for this slot).
fn draw_empty_slot_lane_v2(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    slot_idx: usize,
    page_offset: usize,
    grip_w: f32,
    name_w: f32,
    vol_w: f32,
    mst_w: f32,
    extra_w: f32,
    gap: f32,
    cell_w: f32,
    state: &mut EditorUIState,
    sound_settings: &SoundSettingsState,
    pattern: &SharedPattern,
    plock: &PlockState,
) -> (egui::Response, Option<egui::Pos2>) {
    let inner = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.set_height(LANE_H);

        draw_seq_grip_v2(ui, grip_w, LANE_H);
        let name_response = draw_empty_lane_name_v2(ui, name_w, slot_idx + 1)
            .on_hover_text("Choose an instrument for this slot");
        let add_click_pos = if name_response.clicked() {
            name_response.interact_pointer_pos()
        } else {
            None
        };

        name_response.context_menu(|ui| {
            if ui
                .add_enabled(
                    state.lane_clipboard.is_some(),
                    egui::Button::new("Paste Lane"),
                )
                .clicked()
            {
                if state.paste_lane(setter, params, slot_idx, sound_settings, pattern, plock) {
                    // Flash visual feedback
                    state.slot_flash_until[slot_idx] = ui.ctx().input(|i| i.time) + 0.5;
                }
                ui.close_menu();
            }
        });
        draw_empty_lane_chip_v2(ui, vol_w, "Empty");
        draw_empty_lane_chip_v2(ui, mst_w, "");

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP_TIGHT;
            for local_step in 0..16 {
                let (fill, stroke) = step_colors_v2(
                    ui.ctx(),
                    false,
                    local_step,
                    false,
                    false,
                    false,
                    false,
                    false,
                    true,
                    false,
                    false,
                    false,
                    false,
                );
                let step = page_offset + local_step + 1;
                draw_step_cell_v2(
                    ui,
                    Vec2::new(cell_w, STEP_H),
                    fill,
                    stroke,
                    "",
                    false,
                    false,
                    false,
                    None,
                    true,
                )
                .on_hover_text(format!("Empty slot - step {}", step));
            }
        });

        draw_empty_lane_chip_v2(ui, extra_w, "--");
        draw_empty_lane_chip_v2(ui, extra_w, "--");
        draw_empty_lane_chip_v2(ui, extra_w, "--");
        add_click_pos
    });
    (inner.response, inner.inner)
}

fn draw_empty_lane_name_v2(ui: &mut egui::Ui, width: f32, slot_number: usize) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 21.0), egui::Sense::click());
    let fill = if response.hovered() { P_HOVER } else { PANEL2 };
    ui.painter().rect_filled(rect, RADIUS_CTL, fill);
    ui.painter().rect_stroke(
        rect,
        RADIUS_CTL,
        egui::Stroke::new(1.0, LINE2),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("+{}", slot_number),
        f_mono_sb(11.0),
        FAINT,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn draw_empty_lane_chip_v2(ui: &mut egui::Ui, width: f32, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 21.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, RADIUS_CTL, BG);
    ui.painter().rect_stroke(
        rect,
        RADIUS_CTL,
        egui::Stroke::new(1.0, LINE),
        egui::StrokeKind::Inside,
    );
    if !label.is_empty() {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            f_sans_med(9.5),
            FAINT,
        );
    }
    response
}

fn lane_move_order(from: usize, to: usize) -> [usize; crate::track::MAX_TRACKS] {
    let mut order = std::array::from_fn(|i| i);
    if from >= crate::track::MAX_TRACKS || to >= crate::track::MAX_TRACKS || from == to {
        return order;
    }

    let moved = order[from];
    if from < to {
        for idx in from..to {
            order[idx] = order[idx + 1];
        }
    } else {
        for idx in (to + 1..=from).rev() {
            order[idx] = order[idx - 1];
        }
    }
    order[to] = moved;
    order
}

fn moved_slot_index(order: &[usize; crate::track::MAX_TRACKS], old_idx: usize) -> usize {
    order
        .iter()
        .position(|&idx| idx == old_idx)
        .unwrap_or(old_idx.min(crate::track::MAX_TRACKS - 1))
}

fn remap_slot_index(order: &[usize; crate::track::MAX_TRACKS], slot: usize) -> usize {
    if slot >= crate::track::MAX_TRACKS {
        slot
    } else {
        moved_slot_index(order, slot)
    }
}

fn move_mask_bits(mask: u16, order: &[usize; crate::track::MAX_TRACKS]) -> u16 {
    let mut new_mask = 0u16;
    for (new_idx, &old_idx) in order.iter().enumerate() {
        if (mask & (1u16 << old_idx)) != 0 {
            new_mask |= 1u16 << new_idx;
        }
    }
    new_mask
}

fn set_bool_param_if_changed(setter: &ParamSetter, param: &BoolParam, value: bool) {
    if param.value() != value {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

fn set_float_param_if_changed(setter: &ParamSetter, param: &FloatParam, value: f32) {
    if (param.value() - value).abs() > f32::EPSILON {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

fn set_int_param_if_changed(setter: &ParamSetter, param: &IntParam, value: i32) {
    if param.value() != value {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

fn compute_reorder_gap(
    lane_row_rects: &[Option<egui::Rect>; crate::track::MAX_TRACKS],
    pointer_pos: egui::Pos2,
) -> Option<usize> {
    let mut count = 0usize;
    for rect in lane_row_rects.iter() {
        if let Some(rect) = rect {
            if pointer_pos.y < rect.center().y {
                return Some(count);
            }
            count += 1;
        }
    }
    Some(count)
}

fn draw_lane_reorder_indicator(
    ui: &mut egui::Ui,
    lane_row_rects: &[Option<egui::Rect>; crate::track::MAX_TRACKS],
    gap: usize,
) {
    let top = lane_row_rects.get(gap).and_then(|r| *r);
    let bottom = gap
        .checked_sub(1)
        .and_then(|i| lane_row_rects.get(i))
        .and_then(|r| *r);
    let y = match (top, bottom) {
        (Some(t), Some(_)) => t.top(),
        (Some(t), None) => t.top(),
        (None, Some(b)) => b.bottom(),
        (None, None) => return,
    };
    let x_min = lane_row_rects
        .iter()
        .find_map(|rect| rect.map(|r| r.left()))
        .unwrap_or(0.0);
    let x_max = lane_row_rects
        .iter()
        .find_map(|rect| rect.map(|r| r.right()))
        .unwrap_or(0.0);
    ui.painter().line_segment(
        [egui::pos2(x_min, y), egui::pos2(x_max, y)],
        egui::Stroke::new(2.0, BLUE),
    );
}

#[allow(clippy::too_many_arguments)]
fn handle_lane_reorder_drop(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
    lane_row_rects: &[Option<egui::Rect>; crate::track::MAX_TRACKS],
) {
    let Some(from) = state.lane_drag_source else {
        return;
    };

    if ui.input(|input| input.pointer.primary_down()) {
        return;
    }

    state.lane_drag_source = None;
    let Some(pointer_pos) = ui.input(|input| input.pointer.interact_pos()) else {
        return;
    };
    let Some(gap) = compute_reorder_gap(lane_row_rects, pointer_pos) else {
        return;
    };
    let to = gap.min(crate::track::MAX_TRACKS - 1);

    if from != to {
        apply_lane_reorder_move(
            setter,
            params,
            pattern,
            sound_settings,
            plock,
            state,
            from,
            to,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_lane_reorder_move(
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
    from: usize,
    to: usize,
) {
    if from >= crate::track::MAX_TRACKS || to >= crate::track::MAX_TRACKS || from == to {
        return;
    }

    let order = lane_move_order(from, to);

    let old_step_masks = pattern.step_masks();
    let old_fusions: [Vec<FusedGroup>; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| pattern.load_fusions(slot));
    for (step, mask) in old_step_masks.iter().copied().enumerate() {
        pattern.set_step_mask(step, move_mask_bits(mask, &order));
    }
    for (new_idx, &old_idx) in order.iter().enumerate() {
        pattern.store_fusions(new_idx, &old_fusions[old_idx]);
    }

    let sound_values = sound_settings.read_all();
    let sound_stride = crate::sound_settings::FIELDS_PER_INSTRUMENT_V3;
    let mut new_sound_values = sound_values.clone();
    for (new_idx, &old_idx) in order.iter().enumerate() {
        let dst = new_idx * sound_stride;
        let src = old_idx * sound_stride;
        new_sound_values[dst..dst + sound_stride]
            .copy_from_slice(&sound_values[src..src + sound_stride]);
    }
    sound_settings.write_all(&new_sound_values);

    let old_plock_masks: [u64; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| plock.masks.masks[slot].load(Ordering::Relaxed));
    let old_plock_field_masks: Vec<Vec<u64>> = (0..crate::track::MAX_TRACKS)
        .map(|slot| {
            (0..crate::plock::STEP_COUNT)
                .map(|step| plock.field_masks.get_raw(slot, step))
                .collect()
        })
        .collect();
    let old_plock_values: Vec<Vec<Vec<f32>>> = (0..crate::track::MAX_TRACKS)
        .map(|slot| {
            (0..crate::plock::STEP_COUNT)
                .map(|step| {
                    (0..crate::plock::FIELD_COUNT)
                        .map(|field| plock.values.get(slot, step, field))
                        .collect()
                })
                .collect()
        })
        .collect();
    for (new_idx, &old_idx) in order.iter().enumerate() {
        plock.masks.masks[new_idx].store(old_plock_masks[old_idx], Ordering::Relaxed);
        for step in 0..crate::plock::STEP_COUNT {
            plock
                .field_masks
                .set_raw(new_idx, step, old_plock_field_masks[old_idx][step]);
            for field in 0..crate::plock::FIELD_COUNT {
                plock
                    .values
                    .set(new_idx, step, field, old_plock_values[old_idx][step][field]);
            }
        }
    }

    let seq = &params.seq_plock_state.state;
    let old_seq_masks: [u64; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| seq.masks[slot].load(Ordering::Relaxed));
    let old_seq_probabilities: Vec<Vec<u32>> = (0..crate::track::MAX_TRACKS)
        .map(|slot| {
            (0..crate::plock::STEP_COUNT)
                .map(|step| seq.probabilities[slot][step].load(Ordering::Relaxed))
                .collect()
        })
        .collect();
    let old_seq_stutters: Vec<Vec<u32>> = (0..crate::track::MAX_TRACKS)
        .map(|slot| {
            (0..crate::plock::STEP_COUNT)
                .map(|step| seq.stutters[slot][step].load(Ordering::Relaxed))
                .collect()
        })
        .collect();
    let old_seq_conditions: Vec<Vec<u32>> = (0..crate::track::MAX_TRACKS)
        .map(|slot| {
            (0..crate::plock::STEP_COUNT)
                .map(|step| seq.conditions[slot][step].load(Ordering::Relaxed))
                .collect()
        })
        .collect();
    let old_seq_microtimings: Vec<Vec<u32>> = (0..crate::track::MAX_TRACKS)
        .map(|slot| {
            (0..crate::plock::STEP_COUNT)
                .map(|step| seq.microtimings[slot][step].load(Ordering::Relaxed))
                .collect()
        })
        .collect();
    for (new_idx, &old_idx) in order.iter().enumerate() {
        seq.masks[new_idx].store(old_seq_masks[old_idx], Ordering::Relaxed);
        for step in 0..crate::plock::STEP_COUNT {
            seq.probabilities[new_idx][step]
                .store(old_seq_probabilities[old_idx][step], Ordering::Relaxed);
            seq.stutters[new_idx][step].store(old_seq_stutters[old_idx][step], Ordering::Relaxed);
            seq.conditions[new_idx][step]
                .store(old_seq_conditions[old_idx][step], Ordering::Relaxed);
            seq.microtimings[new_idx][step]
                .store(old_seq_microtimings[old_idx][step], Ordering::Relaxed);
        }
    }

    let mute_values: [bool; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| params.mutes()[slot].value());
    let solo_values: [bool; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| params.solos()[slot].value());
    let mix_values: [bool; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| params.mixes()[slot].value());
    let algo_values: [i32; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| params.algos()[slot].value());
    let humanize_values: [f32; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| params.humanizes()[slot].value());
    let push_values: [f32; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| params.pushes()[slot].value());
    let length_values: [i32; crate::track::MAX_TRACKS] =
        std::array::from_fn(|slot| params.lengths()[slot].value());

    for (new_idx, &old_idx) in order.iter().enumerate() {
        set_bool_param_if_changed(setter, params.mutes()[new_idx], mute_values[old_idx]);
        set_bool_param_if_changed(setter, params.solos()[new_idx], solo_values[old_idx]);
        set_bool_param_if_changed(setter, params.mixes()[new_idx], mix_values[old_idx]);
        set_int_param_if_changed(setter, params.algos()[new_idx], algo_values[old_idx]);
        set_float_param_if_changed(
            setter,
            params.humanizes()[new_idx],
            humanize_values[old_idx],
        );
        set_float_param_if_changed(setter, params.pushes()[new_idx], push_values[old_idx]);
        set_int_param_if_changed(setter, params.lengths()[new_idx], length_values[old_idx]);
    }

    let old_lock_mask = PersistentField::<u16>::map(&params.lane_length_locks, |mask| *mask);
    PersistentField::<u16>::set(
        &params.lane_length_locks,
        move_mask_bits(old_lock_mask, &order),
    );

    let old_selection = state.selected_track_slot;
    let old_selected_instrument = state.selected_instrument;
    let old_fusion_selection = state.fusion_selection_start;
    let old_slot_flash_until = state.slot_flash_until;
    state.selected_track_slot = remap_slot_index(&order, old_selection);
    state.selected_instrument = remap_slot_index(&order, old_selected_instrument);
    state.fusion_selection_start =
        std::array::from_fn(|new_idx| old_fusion_selection[order[new_idx]]);
    state.slot_flash_until = std::array::from_fn(|new_idx| old_slot_flash_until[order[new_idx]]);
    state.fusion_editing = state
        .fusion_editing
        .map(|(slot, group)| (remap_slot_index(&order, slot), group));
    state.plock_popup = state.plock_popup.map(|mut popup| {
        popup.instrument = remap_slot_index(&order, popup.instrument);
        popup
    });
    state.add_module_popup = None;

    let mut new_layout =
        PersistentField::<TrackLayoutState>::map(&params.track_layout, |s| s.clone());
    new_layout.move_slot(from, to);
    PersistentField::<TrackLayoutState>::set(&params.track_layout, new_layout);
}

/// Activate a specific inactive slot with the chosen instrument kind.
/// Triggered from the empty-lane instrument picker.
fn activate_slot(
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    state: &mut EditorUIState,
    slot_idx: usize,
    kind: TrackInstrumentKind,
) {
    if slot_idx >= crate::track::MAX_TRACKS {
        return;
    }
    let mut new_state =
        PersistentField::<TrackLayoutState>::map(&params.track_layout, |s| s.clone());
    if new_state.slots[slot_idx].active {
        return;
    }
    new_state.slots[slot_idx] = TrackSlot::active_with_kind(kind);
    PersistentField::<TrackLayoutState>::set(&params.track_layout, new_state);
    // The slot's settings still hold whatever they were initialized with
    // (legacy defaults of the same index) — align them with the new kind.
    sound_settings.reset_slot_to_defaults(slot_idx, kind);
    select_legacy_track(state, slot_idx);
}

/// Instrument picker popup for an empty lane (opened by the `+N` chip).
fn draw_add_module_popup_if_any(
    ui: &mut egui::Ui,
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    state: &mut EditorUIState,
) {
    let Some(popup) = state.add_module_popup else {
        return;
    };

    // The slot may have been activated in the meantime.
    if params.track_layout.state.is_active(popup.slot) {
        state.add_module_popup = None;
        return;
    }

    let area_id = ui.id().with("add_module_popup");
    let response = egui::Area::new(area_id)
        .kind(egui::UiKind::Popup)
        .order(egui::Order::Foreground)
        .fixed_pos(popup.screen_pos)
        .show(ui.ctx(), |ui| {
            page_menu_frame(ui, BLUE, |ui| {
                page_menu_header(ui, &format!("Slot {} - Add Module", popup.slot + 1), BLUE);
                for kind_idx in 0..TrackInstrumentKind::COUNT {
                    let Some(kind) = TrackInstrumentKind::from_index(kind_idx) else {
                        continue;
                    };
                    if plock_menu_action_row(ui, kind.default_name(), BLUE).clicked() {
                        activate_slot(params, sound_settings, state, popup.slot, kind);
                        state.add_module_popup = None;
                    }
                }
            });
        })
        .response;

    // Close popup when clicking outside.
    let clicked_outside = ui.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .map_or(false, |pos| !response.rect.contains(pos))
    });
    if clicked_outside {
        state.add_module_popup = None;
    }
}

fn draw_page_popup_if_any(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    plock: &PlockState,
    state: &mut EditorUIState,
) {
    let Some(mut popup) = state.page_popup else {
        return;
    };

    let page = popup.page;
    let has_clipboard = state.page_clipboard.is_some();
    let confirm_action = popup.confirm_action;
    let accent = BLUE;

    let area_id = ui.id().with("page_popup");
    let response = egui::Area::new(area_id)
        .kind(egui::UiKind::Popup)
        .order(egui::Order::Foreground)
        .fixed_pos(popup.screen_pos)
        .show(ui.ctx(), |ui| {
            page_menu_frame(ui, accent, |ui| {
                page_menu_header(ui, &format!("Page {}", page + 1), accent);

                if plock_menu_action_row(ui, "Copy", accent).clicked() {
                    state.page_clipboard =
                        Some(copy_page_to_clipboard(pattern, plock, params, page));
                    state.page_popup = None;
                }

                if confirm_action == Some(PageMenuAction::Paste) {
                    ui.label(
                        RichText::new("Overwrite page?")
                            .font(f_sans_med(10.0))
                            .color(INK3),
                    );
                    if plock_menu_action_row(ui, "Yes, overwrite", PL_LINK).clicked() {
                        if let Some(ref clipboard) = state.page_clipboard {
                            paste_page_from_clipboard(pattern, plock, params, page, clipboard);
                            // Auto-extend pattern length so the pasted page is actually played.
                            let required_len = ((page + 1) * 16).clamp(1, 64) as i32;
                            let current_len = params.pattern_length.value().clamp(1, 64) as i32;
                            if required_len > current_len {
                                setter.set_parameter(&params.pattern_length, required_len);
                            }
                        }
                        state.page_popup = None;
                    }
                    if plock_menu_action_row(ui, "No, cancel", INK3).clicked() {
                        state.page_popup = None;
                    }
                } else {
                    let paste_enabled = has_clipboard;
                    let paste_color = if paste_enabled { PL_LINK } else { INK3 };
                    if plock_menu_action_row(ui, "Paste", paste_color).clicked() && paste_enabled {
                        popup.confirm_action = Some(PageMenuAction::Paste);
                        state.page_popup = Some(popup);
                    }
                }

                if confirm_action == Some(PageMenuAction::Clear) {
                    ui.label(
                        RichText::new("Clear page?")
                            .font(f_sans_med(10.0))
                            .color(INK3),
                    );
                    if plock_menu_action_row(ui, "Yes, clear", RED).clicked() {
                        clear_page_for_ui(pattern, plock, params, page);
                        state.page_popup = None;
                    }
                    if plock_menu_action_row(ui, "No, cancel", INK3).clicked() {
                        state.page_popup = None;
                    }
                } else {
                    if plock_menu_action_row(ui, "Clear", RED).clicked() {
                        popup.confirm_action = Some(PageMenuAction::Clear);
                        state.page_popup = Some(popup);
                    }
                }
            });
        })
        .response;

    // Close popup when clicking outside.
    let clicked_outside = ui.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .map_or(false, |pos| !response.rect.contains(pos))
    });
    if clicked_outside {
        state.page_popup = None;
    }
}

fn draw_page_bar_v2(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
    play_page: usize,
    master_length: usize,
) {
    let page_count = (master_length + 15) / 16;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.allocate_ui_with_layout(
            Vec2::new(50.0, CTL_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new("Page").font(f_sans_sb(10.5)).color(INK3));
            },
        );
        for page in 0..4 {
            let enabled = page < page_count.max(1);
            let active = state.current_page == page;
            let response = ui.add_enabled(
                enabled,
                egui::Button::new(
                    RichText::new(format!("{}", page + 1))
                        .monospace()
                        .size(10.5),
                )
                .min_size(Vec2::new(28.0, CTL_HEIGHT))
                .fill(if active { BLUE } else { PANEL2 })
                .stroke(egui::Stroke::new(1.0, if active { BLUE } else { LINE2 }))
                .corner_radius(6.0),
            );
            if play_page == page {
                let led = response.rect.center_bottom() + egui::vec2(0.0, 6.0);
                ui.painter().circle_filled(
                    led,
                    5.0,
                    Color32::from_rgba_unmultiplied(248, 113, 113, 45),
                );
                ui.painter().circle_filled(led, 2.5, RED);
            }
            if response.clicked() {
                state.current_page = page;
            }
            if response.secondary_clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    state.page_popup = Some(PagePopup {
                        page,
                        screen_pos: pos,
                        confirm_action: None,
                    });
                }
            }
        }

        let follow = egui::Button::new(
            RichText::new(if state.follow_mode {
                "Follow ON"
            } else {
                "Follow OFF"
            })
            .font(f_sans_sb(11.0))
            .color(if state.follow_mode {
                Color32::WHITE
            } else {
                INK2
            }),
        )
        .min_size(Vec2::new(78.0, CTL_HEIGHT))
        .fill(if state.follow_mode { BLUE } else { PANEL2 })
        .stroke(egui::Stroke::new(
            1.0,
            if state.follow_mode { BLUE } else { LINE2 },
        ))
        .corner_radius(6.0);
        if ui.add(follow).clicked() {
            state.follow_mode = !state.follow_mode;
        }
        const LEN_GROUP_W: f32 = 468.0;
        const LEN_VALUE_W: f32 = 64.0;
        const PRESET_W: f32 = 104.0;
        const PRESET_LEN_GAP: f32 = 34.0;
        let between_follow_and_len_w = (ui.available_width() - LEN_GROUP_W).max(PRESET_W);
        let len_gap = if between_follow_and_len_w >= PRESET_W + PRESET_LEN_GAP {
            PRESET_LEN_GAP
        } else {
            16.0
        };
        let preset_zone_w = (between_follow_and_len_w - len_gap).max(PRESET_W);
        ui.allocate_ui_with_layout(
            Vec2::new(preset_zone_w, CTL_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.add_space(((preset_zone_w - PRESET_W) * 0.5).max(0.0));
                draw_lane_preset_dropdown(ui, state);
            },
        );
        ui.add_space(len_gap);
        ui.allocate_ui_with_layout(
            Vec2::new(LEN_GROUP_W, CTL_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.label(RichText::new("Len").font(f_sans_sb(10.5)).color(INK3));
                header_param_slider(ui, setter, &params.pattern_length, 132.0, "", false);
                draw_len_value_fixed(ui, master_length, LEN_VALUE_W);
                for &len in &[16, 32, 48, 64] {
                    let active = master_length == len;
                    let btn =
                        egui::Button::new(RichText::new(format!("{}", len)).monospace().size(10.5))
                            .min_size(Vec2::new(36.0, CTL_HEIGHT))
                            .fill(if active { BLUE } else { PANEL2 })
                            .stroke(egui::Stroke::new(1.0, if active { BLUE } else { LINE2 }))
                            .corner_radius(6.0);
                    if ui.add(btn).clicked() {
                        setter.set_parameter(&params.pattern_length, len as i32);
                    }
                }
                let can_double = master_length <= 32;
                let x2 = egui::Button::new(RichText::new("x2").monospace().size(10.5))
                    .min_size(Vec2::new(36.0, CTL_HEIGHT))
                    .fill(PANEL2)
                    .stroke(egui::Stroke::new(1.0, LINE2))
                    .corner_radius(6.0);
                if ui.add_enabled(can_double, x2).clicked() {
                    for i in 0..master_length {
                        pattern.set_step_mask(master_length + i, pattern.load_step_mask(i));
                        for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
                            if plock.masks.is_active(inst, i) {
                                let field_mask = plock.field_masks.get_raw(inst, i);
                                plock.masks.set_active(inst, master_length + i, true);
                                plock
                                    .field_masks
                                    .set_raw(inst, master_length + i, field_mask);
                                for field in 0..crate::plock::FIELD_COUNT {
                                    let value = plock.values.get(inst, i, field);
                                    plock.values.set(inst, master_length + i, field, value);
                                }
                            }
                        }
                    }
                    duplicate_fusions_for_x2(pattern, params, plock, master_length);
                    setter.set_parameter(&params.pattern_length, (master_length * 2) as i32);
                }
            },
        );
    });
    draw_lane_preset_warning_if_any(ui, params, sound_settings, pattern, state);
}

fn draw_lane_preset_dropdown(ui: &mut egui::Ui, state: &mut EditorUIState) {
    egui::ComboBox::from_id_salt("lane_preset_dropdown")
        .selected_text(RichText::new("Preset").font(f_sans_sb(10.5)).color(INK2))
        .width(94.0)
        .show_ui(ui, |ui| {
            ui.set_min_width(132.0);
            if ui
                .selectable_label(
                    false,
                    RichText::new("Clear All").font(f_sans_med(11.0)).color(RED),
                )
                .clicked()
            {
                state.lane_preset_confirm = Some(LanePresetAction::ClearAll);
                ui.close_menu();
            }
            if ui
                .selectable_label(false, RichText::new("Preset 4").font(f_sans_med(11.0)))
                .clicked()
            {
                state.lane_preset_confirm = Some(LanePresetAction::Preset4);
                ui.close_menu();
            }
            if ui
                .selectable_label(false, RichText::new("Preset 12").font(f_sans_med(11.0)))
                .clicked()
            {
                state.lane_preset_confirm = Some(LanePresetAction::Preset12);
                ui.close_menu();
            }
        });
}

fn draw_lane_preset_warning_if_any(
    ui: &mut egui::Ui,
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
) {
    let Some(action) = state.lane_preset_confirm else {
        return;
    };

    let screen_rect = ui.ctx().screen_rect();
    let panel_w = 318.0;
    let pos = egui::pos2(
        screen_rect.center().x - panel_w * 0.5,
        screen_rect.top() + 92.0,
    );
    egui::Area::new(ui.id().with("lane_preset_warning"))
        .kind(egui::UiKind::Popup)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(P_ACTIVE)
                .corner_radius(RADIUS_PANEL)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(panel_w);
                    ui.label(RichText::new("Warning").font(f_sans_sb(12.0)).color(RED));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "{} modifies the current pattern and lane layout.",
                            action.label()
                        ))
                        .font(f_sans_med(10.5))
                        .color(INK2),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let apply = egui::Button::new(
                            RichText::new(action.apply_label())
                                .font(f_sans_sb(10.5))
                                .color(Color32::WHITE),
                        )
                        .min_size(Vec2::new(128.0, CTL_HEIGHT))
                        .fill(RED)
                        .stroke(egui::Stroke::new(1.0, RED))
                        .corner_radius(6.0);
                        if ui.add(apply).clicked() {
                            apply_lane_preset_action(
                                params,
                                sound_settings,
                                pattern,
                                state,
                                action,
                            );
                            state.lane_preset_confirm = None;
                        }

                        let cancel = egui::Button::new(
                            RichText::new("Cancel").font(f_sans_sb(10.5)).color(INK2),
                        )
                        .min_size(Vec2::new(82.0, CTL_HEIGHT))
                        .fill(PANEL2)
                        .stroke(egui::Stroke::new(1.0, LINE2))
                        .corner_radius(6.0);
                        if ui.add(cancel).clicked() {
                            state.lane_preset_confirm = None;
                        }
                    });
                });
        });
}

fn draw_len_value_fixed(ui: &mut egui::Ui, master_length: usize, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, CTL_HEIGHT), egui::Sense::hover());
    let number_text = format!("{:>2}", master_length);
    let number_pos = egui::pos2(rect.left(), rect.center().y);
    ui.painter().text(
        number_pos,
        egui::Align2::LEFT_CENTER,
        number_text,
        f_mono(12.0),
        INK,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 23.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "steps",
        f_sans(9.5),
        INK3,
    );
}

fn draw_seq_header_v2(
    ui: &mut egui::Ui,
    page_offset: usize,
    play_step: usize,
    grip_w: f32,
    name_w: f32,
    vol_w: f32,
    mst_w: f32,
    extra_w: f32,
    gap: f32,
    cell_w: f32,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.add_sized(Vec2::new(grip_w, 16.0), egui::Label::new(""));
        ui.add_sized(Vec2::new(name_w, 16.0), egui::Label::new(""));
        ui.add_sized(
            Vec2::new(vol_w, 16.0),
            egui::Label::new(RichText::new("Vol").font(f_sans_sb(9.5)).color(INK3)),
        );
        // M / S / T column headings (aligned with the lane tags below)
        ui.allocate_ui(Vec2::new(mst_w, 16.0), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP_TIGHT;
                for t in ["M", "S", "T"] {
                    ui.add_sized(
                        Vec2::new(STEP_H, 16.0),
                        egui::Label::new(RichText::new(t).font(f_mono(9.0)).color(INK3)),
                    );
                }
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP_TIGHT;
            for local in 0..16 {
                let step = page_offset + local;
                let color = if play_step == step {
                    BLUE
                } else if local % 4 == 0 {
                    INK2
                } else {
                    FAINT
                };
                ui.add_sized(
                    Vec2::new(cell_w, 16.0),
                    egui::Label::new(
                        RichText::new(format!("{}", step + 1))
                            .font(f_mono(9.0))
                            .color(color),
                    ),
                );
            }
        });
        for label in ["Hum", "Push", "Len"] {
            ui.add_sized(
                Vec2::new(extra_w, 16.0),
                egui::Label::new(RichText::new(label).font(f_sans_sb(9.5)).color(INK3)),
            );
        }
    });
}

fn draw_seq_grip_v2(ui: &mut egui::Ui, width: f32, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::drag());
    // 2x3 dot matrix (drag-handle look; avoids relying on braille glyph coverage)
    let c = rect.center();
    let dot_color = if response.dragged() || response.hovered() {
        INK2
    } else {
        FAINT
    };
    for col in 0..2 {
        for row in 0..3 {
            let p = egui::pos2(
                c.x + (col as f32 - 0.5) * 4.0,
                c.y + (row as f32 - 1.0) * 3.0,
            );
            ui.painter().circle_filled(p, 1.0, dot_color);
        }
    }
    response
}

fn draw_lane_name_v2(ui: &mut egui::Ui, width: f32, selected: bool, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 21.0), egui::Sense::click());
    let fill = if selected {
        BLUE
    } else if response.hovered() {
        P_HOVER
    } else {
        PANEL2
    };
    ui.painter().rect_filled(rect, RADIUS_CTL, fill);
    // Borderless at rest; only the selected lane gets a blue outline.
    if selected {
        ui.painter().rect_stroke(
            rect,
            RADIUS_CTL,
            egui::Stroke::new(1.0, BLUE),
            egui::StrokeKind::Inside,
        );
    }
    let text_color = if selected {
        Color32::WHITE
    } else if response.hovered() {
        INK
    } else {
        INK2
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        f_mono_sb(11.0),
        text_color,
    );
    response
}

fn draw_tag_button_v2(
    ui: &mut egui::Ui,
    label: &str,
    color: Color32,
    text_on: Color32,
    active: bool,
    tooltip: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(TAG_SIZE), egui::Sense::click());
    let fill = if active { color } else { PANEL2 };
    let text = if active { text_on } else { FAINT };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter().rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, if active { color } else { LINE2 }),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        f_mono_sb(9.0),
        text,
    );
    if tooltip.is_empty() {
        response
    } else {
        response.on_hover_text(tooltip)
    }
}

fn draw_tag_param_v2(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &BoolParam,
    label: &str,
    color: Color32,
    text_on: Color32,
    tooltip: &str,
) -> egui::Response {
    let value = param.value();
    let response = draw_tag_button_v2(ui, label, color, text_on, value, tooltip);
    if response.clicked() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !value);
        setter.end_set_parameter(param);
    }
    response
}

fn draw_step_cell_v2(
    ui: &mut egui::Ui,
    size: Vec2,
    fill: Color32,
    stroke: egui::Stroke,
    text: &str,
    enabled: bool,
    playhead: bool,
    is_fusion_mid: bool,
    fusion_span: Option<usize>,
    dashed_border: bool,
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);

    // Fusion block: the start cell draws one continuous rectangle spanning all
    // merged cells (including the gaps between them). Mid cells stay transparent
    // so the start cell's block remains unbroken.
    let block_rect = if let Some(span) = fusion_span {
        let mut r = rect;
        let extra = (size.x + GAP_TIGHT) * (span.saturating_sub(1) as f32);
        r.set_right(r.right() + extra);
        r
    } else {
        rect
    };

    let stroke = if response.hovered() && enabled && !is_fusion_mid {
        egui::Stroke::new(1.0, BLUE)
    } else {
        stroke
    };

    if !is_fusion_mid {
        ui.painter().rect_filled(block_rect, 4.0, fill);
        if stroke.width > 0.0 && stroke.color.a() > 0 {
            if dashed_border {
                draw_dashed_rect(ui.painter(), block_rect.shrink(0.5));
            } else {
                ui.painter()
                    .rect_stroke(block_rect, 4.0, stroke, egui::StrokeKind::Inside);
            }
        }
    }

    // Playhead: an inset white ring drawn ON TOP of the state border.
    if playhead {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            4.0,
            egui::Stroke::new(1.5, white_a(153)),
            egui::StrokeKind::Inside,
        );
    }
    if !text.is_empty() {
        ui.painter().text(
            block_rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            f_mono_sb(10.0),
            Color32::WHITE,
        );
    }
    response
}

fn draw_dashed_rect(painter: &egui::Painter, rect: egui::Rect) {
    let stroke = egui::Stroke::new(2.5, Color32::from_rgba_unmultiplied(0, 0, 0, 235));
    let dash = 6.0;
    let gap = 3.0;
    let step = dash + gap;
    let corner = 4.0;

    let mut x = rect.left() + corner;
    while x < rect.right() - corner {
        let end = (x + dash).min(rect.right() - corner);
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(end, rect.top())],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(x, rect.bottom()), egui::pos2(end, rect.bottom())],
            stroke,
        );
        x += step;
    }

    let mut y = rect.top() + corner;
    while y < rect.bottom() - corner {
        let end = (y + dash).min(rect.bottom() - corner);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.left(), end)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(rect.right(), y), egui::pos2(rect.right(), end)],
            stroke,
        );
        y += step;
    }
}

fn step_colors_v2(
    ctx: &egui::Context,
    sequencer_mode: bool,
    local_step: usize,
    active: bool,
    has_sound_plock: bool,
    is_snapshot: bool,
    has_seq_plock: bool,
    is_current: bool,
    disabled: bool,
    selection_start: bool,
    is_fusion_start: bool,
    is_fusion_mid: bool,
    is_editing: bool,
) -> (Color32, egui::Stroke) {
    if disabled {
        return (
            Color32::from_rgb(10, 10, 14),
            egui::Stroke::new(1.0, Color32::from_rgb(0, 0, 0)),
        );
    }
    if selection_start {
        return (Color32::from_rgb(20, 34, 58), egui::Stroke::new(1.5, BLUE));
    }
    if is_fusion_start && !is_editing {
        return (Color32::from_rgb(20, 34, 58), egui::Stroke::new(1.0, BLUE));
    }
    if is_fusion_mid && !is_editing {
        // Mid cells are visually merged into the start cell's continuous block.
        return (Color32::TRANSPARENT, egui::Stroke::NONE);
    }

    let empty = if local_step % 4 == 0 {
        Color32::from_rgb(35, 35, 44)
    } else {
        Color32::from_rgb(27, 27, 34)
    };
    // Playhead border is NOT applied here — draw_step_cell_v2 paints an inset
    // white ring on top so the state's own border colour is preserved.
    let (fill, border) = if sequencer_mode {
        if active && has_seq_plock {
            (SEQPL, SEQPL)
        } else if has_seq_plock {
            (Color32::from_rgb(28, 18, 48), SEQPL_DIM)
        } else if active {
            (BLUE, BLUE)
        } else if is_current {
            (Color32::from_rgb(48, 48, 60), LINE)
        } else {
            (empty, LINE)
        }
    } else if active && has_sound_plock {
        if is_snapshot {
            (PL_SNAP, PL_SNAP)
        } else {
            (PL_LINK, PL_LINK)
        }
    } else if active {
        (BLUE, BLUE)
    } else if has_sound_plock {
        if is_snapshot {
            (Color32::from_rgb(36, 16, 16), PL_SNAP_DIM)
        } else {
            (Color32::from_rgb(36, 26, 8), PL_LINK_DIM)
        }
    } else if is_current {
        (Color32::from_rgb(48, 48, 60), LINE)
    } else {
        (empty, LINE)
    };

    if is_editing {
        let time = ctx.input(|i| i.time) as f32;
        let pulse = ((time * 4.0).sin() + 1.0) * 0.5;
        // When editing a fused group, pulse every cell of the group using the
        // same blue fusion base so the block flashes as one unit.
        let fusion_fill = if is_fusion_start || is_fusion_mid {
            Color32::from_rgb(20, 34, 58)
        } else {
            fill
        };
        let edit_color = Color32::from_rgba_unmultiplied(
            (BLUE.r() as f32 * pulse + fusion_fill.r() as f32 * (1.0 - pulse)) as u8,
            (BLUE.g() as f32 * pulse + fusion_fill.g() as f32 * (1.0 - pulse)) as u8,
            (BLUE.b() as f32 * pulse + fusion_fill.b() as f32 * (1.0 - pulse)) as u8,
            255,
        );
        return (edit_color, egui::Stroke::new(1.5, BLUE));
    }

    (fill, egui::Stroke::new(1.0, border))
}

fn draw_mini_value_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    width: f32,
    fill: Color32,
    tooltip: &str,
) -> egui::Response {
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width, 17.0), egui::Sense::click_and_drag());
    if let Some(pos) = response.interact_pointer_pos() {
        if response.clicked() || response.dragged() {
            let t = egui::emath::remap_clamp(pos.x, rect.x_range(), 0.0..=1.0);
            *value = min + (max - min) * t;
            response.mark_changed();
        }
    }
    let track = egui::Rect::from_center_size(rect.center(), Vec2::new(width, 6.0));
    ui.painter().rect_filled(track, 5.0, PANEL2);
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let mut fill_rect = track;
    fill_rect.set_right(track.left() + track.width() * t);
    ui.painter().rect_filled(fill_rect, 5.0, fill);
    if tooltip.is_empty() {
        response
    } else {
        response.on_hover_text(tooltip)
    }
}

fn draw_param_mini_slider_with_value(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &FloatParam,
    min: f32,
    max: f32,
    width: f32,
    fill: Color32,
    tooltip: &str,
    format: impl Fn(f32) -> String,
) -> egui::Response {
    let mut value = param.value();
    let response = draw_mini_value_slider(ui, &mut value, min, max, width, fill, "");
    if response.double_clicked() {
        value = param.default_plain_value().clamp(min, max);
        setter.set_parameter(param, value);
    } else if response.changed() {
        setter.set_parameter(param, value.clamp(min, max));
    }
    if response.hovered() || response.dragged() {
        draw_mini_slider_value_tooltip(ui, &response, &format!("{}: {}", tooltip, format(value)));
    }
    response
}

fn draw_mini_slider_value_tooltip(ui: &egui::Ui, response: &egui::Response, text: &str) {
    let mut pos = response.rect.center_top() + egui::vec2(0.0, -7.0);
    if let Some(to_global) = ui.ctx().layer_transform_to_global(ui.layer_id()) {
        pos = to_global * pos;
    }
    egui::Area::new(response.id.with("mini_slider_value_tooltip"))
        .kind(egui::UiKind::Tooltip)
        .order(egui::Order::Foreground)
        .pivot(egui::Align2::CENTER_BOTTOM)
        .fixed_pos(pos)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(P_ACTIVE)
                .stroke(egui::Stroke::new(1.0, LINE2))
                .corner_radius(6.0)
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 5,
                    bottom: 5,
                })
                .show(ui, |ui| {
                    ui.label(RichText::new(text).font(f_mono_med(10.5)).color(INK));
                });
        });
}

fn p_lock_mode_segmented(ui: &mut egui::Ui, selected: usize) -> usize {
    text_segmented(
        ui,
        "plock_mode",
        &[("Sound", PL_LINK), ("Sequencer", SEQPL)],
        selected,
    )
}

fn generator_song_segmented(ui: &mut egui::Ui, selected: usize) -> usize {
    text_segmented(
        ui,
        "gen_song_mode",
        &[("Generator", BLUE), ("Song", PL_LINK)],
        selected,
    )
}

fn text_segmented(
    ui: &mut egui::Ui,
    id_salt: &'static str,
    options: &[(&str, Color32)],
    selected: usize,
) -> usize {
    let font = f_sans_sb(10.5);
    let padding = 24.0; // 12 left + 12 right
    let mut widths = Vec::with_capacity(options.len());
    for (label, _) in options {
        let tw = ui.fonts(|f| {
            f.layout_no_wrap((*label).to_string(), font.clone(), INK)
                .size()
                .x
        });
        widths.push((tw + padding).max(56.0));
    }
    let total_w: f32 = widths.iter().sum();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, CTL_HEIGHT), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, PANEL2);
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, LINE2),
        egui::StrokeKind::Inside,
    );

    let mut result = selected.min(options.len().saturating_sub(1));
    let mut x = rect.left();
    for (idx, (label, accent)) in options.iter().enumerate() {
        let seg = egui::Rect::from_min_size(
            egui::pos2(x, rect.top()),
            Vec2::new(widths[idx], CTL_HEIGHT),
        );
        let active = idx == result;
        let response = ui.interact(
            seg,
            ui.make_persistent_id((id_salt, idx)),
            egui::Sense::click(),
        );
        if active {
            painter.rect_filled(seg.shrink(1.0), 5.0, *accent);
        } else if response.hovered() {
            painter.rect_filled(seg.shrink(1.0), 5.0, P_HOVER);
            painter.rect_stroke(
                seg.shrink(1.0),
                5.0,
                egui::Stroke::new(1.0, *accent),
                egui::StrokeKind::Inside,
            );
        }
        if idx > 0 {
            painter.line_segment(
                [
                    egui::pos2(seg.left(), rect.top() + 3.0),
                    egui::pos2(seg.left(), rect.bottom() - 3.0),
                ],
                egui::Stroke::new(1.0, LINE2),
            );
        }
        painter.text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if active {
                Color32::WHITE
            } else if response.hovered() {
                INK
            } else {
                INK2
            },
        );
        if response.clicked() {
            result = idx;
            ui.ctx().request_repaint();
        }
        x += widths[idx];
    }

    result
}

fn led_toggle(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let font = f_sans_sb(11.0);
    let led_r = 3.5;
    let padding_x = 12.0;
    let gap = 7.0;
    let label_w = ui.fonts(|f| {
        f.layout_no_wrap(label.to_string(), font.clone(), INK)
            .size()
            .x
    });
    let total_w = padding_x + led_r * 2.0 + gap + label_w + padding_x;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(total_w, CTL_HEIGHT), egui::Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter_at(rect);

    let fill = if active { BLUE_GLOW } else { PANEL2 };
    let stroke_color = if active {
        BLUE
    } else if hovered {
        BLUE
    } else {
        LINE2
    };
    let text_color = if active {
        Color32::WHITE
    } else if hovered {
        INK
    } else {
        INK2
    };

    painter.rect_filled(rect, 6.0, fill);
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, stroke_color),
        egui::StrokeKind::Inside,
    );

    let led_center = egui::pos2(rect.left() + padding_x + led_r, rect.center().y);
    let led_color = if active { BLUE } else { FAINT };
    painter.circle_filled(led_center, led_r, led_color);
    if active {
        painter.circle_filled(
            led_center,
            led_r + 2.0,
            Color32::from_rgba_premultiplied(BLUE.r(), BLUE.g(), BLUE.b(), 45),
        );
    }

    painter.text(
        egui::pos2(led_center.x + led_r + gap, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        text_color,
    );

    response
}

fn normalize_slider_value(value: f32, min: f32, max: f32, logarithmic: bool) -> f32 {
    if logarithmic && min > 0.0 && max > min {
        let min_log = min.ln();
        let max_log = max.ln();
        ((value.max(min).ln() - min_log) / (max_log - min_log)).clamp(0.0, 1.0)
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    }
}

fn denormalize_slider_value(norm: f32, min: f32, max: f32, logarithmic: bool) -> f32 {
    let norm = norm.clamp(0.0, 1.0);
    if logarithmic && min > 0.0 && max > min {
        (min.ln() + norm * (max.ln() - min.ln())).exp()
    } else {
        min + norm * (max - min)
    }
}

fn format_editor_value(value: f32, suffix: Option<&str>) -> String {
    let number = if value.abs() >= 100.0 {
        format!("{:.0}", value)
    } else if value.abs() >= 10.0 {
        format!("{:.1}", value)
    } else {
        format!("{:.2}", value)
    };
    match suffix {
        Some(s) => format!("{}{}", number, s),
        None => number,
    }
}

/// Left-aligned label in the fixed 138px column so every editor row aligns.
fn editor_label(ui: &mut egui::Ui, text: &str) {
    ui.allocate_ui_with_layout(
        Vec2::new(EDITOR_LABEL_W, 22.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(RichText::new(text).font(f_sans_med(11.5)).color(INK2));
        },
    );
}

fn draw_editor_slider_track(
    ui: &mut egui::Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    logarithmic: bool,
    track_w: f32,
) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(
        Vec2::new(track_w.max(60.0), 22.0),
        egui::Sense::click_and_drag(),
    );
    if let Some(pos) = response.interact_pointer_pos() {
        if response.clicked() || response.dragged() {
            let norm = egui::emath::remap_clamp(pos.x, rect.x_range(), 0.0..=1.0);
            *value = denormalize_slider_value(norm, min, max, logarithmic).clamp(min, max);
            response.mark_changed();
        }
    }

    let track = egui::Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 6.0));
    ui.painter().rect_filled(track, 3.0, PANEL2);
    let norm = normalize_slider_value(*value, min, max, logarithmic);
    if norm > 0.0 {
        let mut fill = track;
        fill.set_right(track.left() + track.width() * norm);
        ui.painter().rect_filled(fill, 3.0, BLUE);
    }
    if response.hovered() || response.dragged() {
        let x = track.left() + track.width() * norm;
        ui.painter().circle_filled(
            egui::pos2(x, track.center().y),
            5.5,
            Color32::from_rgb(238, 242, 248),
        );
    }

    response
}

fn draw_note_freq_mode_toggle(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    notes_active: bool,
) -> Option<bool> {
    let width = 78.0;
    let height = 22.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, PANEL2);
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, LINE2),
        egui::StrokeKind::Inside,
    );

    let mut result = None;
    for (idx, label) in ["Hz", "Note"].iter().enumerate() {
        let active = (idx == 1) == notes_active;
        let seg = egui::Rect::from_min_size(
            egui::pos2(rect.left() + idx as f32 * width * 0.5, rect.top()),
            Vec2::new(width * 0.5, height),
        );
        let response = ui.interact(
            seg,
            ui.make_persistent_id(("note_freq_mode", &id_salt, idx)),
            egui::Sense::click(),
        );
        if active {
            painter.rect_filled(seg.shrink(1.0), 5.0, BLUE);
        } else if response.hovered() {
            painter.rect_stroke(
                seg.shrink(1.0),
                5.0,
                egui::Stroke::new(1.0, BLUE),
                egui::StrokeKind::Inside,
            );
        }
        if idx == 1 {
            painter.line_segment(
                [
                    egui::pos2(seg.left(), rect.top() + 3.0),
                    egui::pos2(seg.left(), rect.bottom() - 3.0),
                ],
                egui::Stroke::new(1.0, LINE2),
            );
        }
        painter.text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            f_sans_sb(10.5),
            if active {
                Color32::WHITE
            } else if response.hovered() {
                INK
            } else {
                INK2
            },
        );
        if response.clicked() && !active {
            result = Some(idx == 1);
        }
    }

    result
}

fn draw_note_step_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).font(f_mono_sb(12.0)).color(INK2))
            .min_size(Vec2::new(24.0, 22.0))
            .fill(PANEL2)
            .stroke(egui::Stroke::new(1.0, LINE2))
            .corner_radius(5.0),
    )
}

struct EditorFrequencyRowResult {
    response: egui::Response,
    value_changed: bool,
    mode_change: Option<bool>,
}

fn draw_editor_frequency_row(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    logarithmic: bool,
    ratio: f32,
    notes_active: bool,
) -> EditorFrequencyRowResult {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, label);

        let mut value_changed = false;
        let mut row_response = ui.allocate_response(Vec2::ZERO, egui::Sense::hover());
        let mode_w = 78.0;

        if notes_active {
            let note_val = freq_to_note(*value * ratio).round();
            if draw_note_step_button(ui, "-").clicked() {
                let new_note = (note_val - 1.0).max(0.0);
                *value = note_to_freq(new_note) / ratio;
                value_changed = true;
            }
            ui.allocate_ui_with_layout(
                Vec2::new(58.0, 22.0),
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    ui.label(
                        RichText::new(note_name(note_val))
                            .font(f_mono_sb(13.0))
                            .color(INK),
                    );
                },
            );
            if draw_note_step_button(ui, "+").clicked() {
                let new_note = (note_val + 1.0).min(127.0);
                *value = note_to_freq(new_note) / ratio;
                value_changed = true;
            }
            let used_w = 24.0 + 8.0 + 58.0 + 8.0 + 24.0;
            ui.add_space((ui.available_width() - mode_w - 8.0 - used_w).max(0.0));
        } else {
            let track_w = (ui.available_width() - EDITOR_VALUE_W - 8.0 - mode_w - 8.0).max(60.0);
            row_response = draw_editor_slider_track(ui, value, min, max, logarithmic, track_w);
            value_changed = row_response.changed();
            ui.allocate_ui_with_layout(
                Vec2::new(EDITOR_VALUE_W, 22.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(format_editor_value(*value, Some("Hz")))
                            .font(f_mono(11.0))
                            .color(INK),
                    );
                },
            );
        }

        let mode_change = draw_note_freq_mode_toggle(ui, id_salt, notes_active);
        EditorFrequencyRowResult {
            response: row_response,
            value_changed,
            mode_change,
        }
    })
    .inner
}

fn draw_editor_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    logarithmic: bool,
    suffix: Option<&str>,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, label);

        // Track flexes to fill the fixed-width params column.
        let track_w = (ui.available_width() - EDITOR_VALUE_W - 8.0).max(60.0);
        let response = draw_editor_slider_track(ui, value, min, max, logarithmic, track_w);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format_editor_value(*value, suffix))
                    .font(f_mono(11.0))
                    .color(INK),
            );
        });
        response
    })
    .inner
}

fn draw_editor_switch_row(ui: &mut egui::Ui, label: &str, value: &mut f32) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, label);
        // Push the switch to the row's right edge (.ctl--switch: space-between).
        let avail = ui.available_width();
        ui.add_space((avail - 34.0).max(0.0));
        let checked = *value >= 0.5;
        let mut response = ui.add(ToggleSwitch::new(checked));
        if response.clicked() {
            *value = if checked { 0.0 } else { 1.0 };
            response.mark_changed();
        }
        response
    })
    .inner
}

// ---------------------------------------------------------------------------------------------------------------
fn draw_track_tab(
    ui: &mut egui::Ui,
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    setter: &ParamSetter,
    state: &mut EditorUIState,
) {
    let slot_idx = state.selected_track_slot;
    let layout_state =
        PersistentField::<TrackLayoutState>::map(&params.track_layout, |s| s.clone());
    let slot = &layout_state.slots[slot_idx];

    if !slot.active {
        ui.add_space(12.0);
        ui.label(
            RichText::new("Empty slot - click a +N chip in the grid to add a module.")
                .font(f_sans_med(11.0))
                .color(INK3),
        );
        return;
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new(format!("Slot {} — {}", slot_idx + 1, slot.name))
            .font(f_sans_sb(12.0))
            .color(Color32::WHITE),
    );

    ui.add_space(12.0);
    ui.label(
        RichText::new("Instrument")
            .font(f_sans_med(10.5))
            .color(INK3),
    );
    let kinds = [
        TrackInstrumentKind::Kick,
        TrackInstrumentKind::Snare,
        TrackInstrumentKind::HiHat,
        TrackInstrumentKind::OpenHiHat,
        TrackInstrumentKind::Tom,
        TrackInstrumentKind::Clap,
        TrackInstrumentKind::Ride,
        TrackInstrumentKind::Cymbal,
        TrackInstrumentKind::Snare606,
        TrackInstrumentKind::BassDrum808,
        TrackInstrumentKind::Perc1,
    ];
    let current_kind = slot.kind;
    let current_label = current_kind.default_name();
    egui::ComboBox::from_id_salt("track_kind")
        .selected_text(
            RichText::new(current_label)
                .font(f_sans_med(11.0))
                .color(Color32::WHITE),
        )
        .width(180.0)
        .show_ui(ui, |ui| {
            for kind in kinds {
                let label = kind.default_name();
                if ui
                    .selectable_label(
                        kind == current_kind,
                        RichText::new(label).font(f_sans_med(11.0)),
                    )
                    .clicked()
                    && kind != current_kind
                {
                    let mut new_state = layout_state.clone();
                    new_state.slots[slot_idx].kind = kind;
                    new_state.slots[slot_idx].name = label.to_string();
                    new_state.slots[slot_idx].midi_note = kind.default_midi_note();
                    PersistentField::<TrackLayoutState>::set(&params.track_layout, new_state);
                    // New kind → new voice: align the slot's settings with the
                    // new instrument's defaults (the audio thread reinitializes
                    // the voice via last_slot_kinds detection).
                    sound_settings.reset_slot_to_defaults(slot_idx, kind);
                    // Keep the selection on this slot; the Sound tab schema
                    // follows the slot's kind automatically.
                    state.selected_instrument = slot_idx;
                }
            }
        });

    ui.add_space(16.0);
    ui.label(RichText::new("Routing").font(f_sans_med(10.5)).color(INK3));
    let mut new_state = layout_state.clone();
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut main_on = slot.routing.main_on;
        if ui
            .checkbox(&mut main_on, RichText::new("Main").font(f_sans_med(11.0)))
            .changed()
        {
            new_state.slots[slot_idx].routing.main_on = main_on;
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Out").font(f_sans_med(11.0)).color(INK3));
        let out_options: Vec<(u8, &str)> = std::iter::once((0u8, "No Aux"))
            .chain((1..=crate::track::MAX_TRACKS as u8).map(|i| (i, "Out")))
            .collect();
        let current_out = slot.routing.out_select.index();
        egui::ComboBox::from_id_salt("track_out")
            .selected_text(
                RichText::new(if current_out == 0 {
                    "No Aux".to_string()
                } else {
                    format!("Out {}", current_out)
                })
                .font(f_sans_med(11.0))
                .color(Color32::WHITE),
            )
            .width(120.0)
            .show_ui(ui, |ui| {
                for (idx, _) in &out_options {
                    let label = if *idx == 0 {
                        "No Aux".to_string()
                    } else {
                        format!("Out {}", idx)
                    };
                    if ui
                        .selectable_label(
                            current_out == *idx,
                            RichText::new(&label).font(f_sans_med(11.0)),
                        )
                        .clicked()
                        && current_out != *idx
                    {
                        new_state.assign_slot_output_exclusive(
                            slot_idx,
                            crate::track::TrackAudioOut::from_index(*idx),
                        );
                        changed = true;
                    }
                }
            });
    });

    ui.add_space(16.0);
    ui.label(
        RichText::new("MIDI Note")
            .font(f_sans_med(10.5))
            .color(INK3),
    );
    ui.horizontal(|ui| {
        let mut note = slot.midi_note as i32;
        if ui
            .add(egui::DragValue::new(&mut note).range(0..=127).speed(1.0))
            .changed()
        {
            new_state.slots[slot_idx].midi_note = note.clamp(0, 127) as u8;
            changed = true;
        }
    });

    if changed {
        PersistentField::<TrackLayoutState>::set(&params.track_layout, new_state);
    }

    // Per-track sequencing params kept out of the grid only where useful.
    ui.add_space(16.0);
    ui.label(
        RichText::new("Sequencing")
            .font(f_sans_med(10.5))
            .color(INK3),
    );
    let master_length = params.pattern_length.value().clamp(1, 64) as usize;
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(70.0, 20.0),
            egui::Label::new(RichText::new("Length").font(f_sans_med(11.0)).color(INK2)),
        );
        draw_track_length_control(
            ui,
            setter,
            params,
            params.lengths()[slot_idx],
            slot_idx,
            master_length,
        );
    });
}

fn apply_lane_layout_preset(
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
    layout: TrackLayoutState,
    clear_pattern_data: bool,
) {
    if clear_pattern_data {
        load_pattern_for_ui(pattern, &crate::sequencer::pattern::Pattern::empty());
        params.plock_state.state.clear_all();
        params.seq_plock_state.state.clear_all();
        clear_all_fusions(pattern);
        state.last_loaded_slot = None;
    }

    for (slot_idx, slot) in layout.slots.iter().enumerate() {
        if slot.active {
            sound_settings.reset_slot_to_defaults(slot_idx, slot.kind);
        }
    }

    let selected_slot = layout.active_slot_indices().next().unwrap_or(0);
    PersistentField::<TrackLayoutState>::set(&params.track_layout, layout);
    state.add_module_popup = None;
    select_legacy_track(state, selected_slot);
}

fn apply_lane_preset_action(
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
    action: LanePresetAction,
) {
    let (layout, clear_pattern_data) = match action {
        LanePresetAction::ClearAll => (TrackLayoutState::empty_layout(), true),
        LanePresetAction::Preset4 => (TrackLayoutState::modular_default_layout(), false),
        LanePresetAction::Preset12 => (TrackLayoutState::preset_12_layout(), false),
    };
    apply_lane_layout_preset(
        params,
        sound_settings,
        pattern,
        state,
        layout,
        clear_pattern_data,
    );
}

// Sound Panel (always visible, tabbed by instrument)
// ---------------------------------------------------------------------------------------------------------------
fn draw_sound_panel(
    ui: &mut egui::Ui,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    setter: &ParamSetter,
    state: &mut EditorUIState,
) {
    state.selected_instrument = state.selected_instrument.min(crate::track::MAX_TRACKS - 1);
    state.selected_track_slot = state.selected_instrument;
    // Slot index drives per-slot state (sound_settings, algos, mutes, ...);
    // the voice index drives registry/schema lookups (INSTRUMENTS, special_param).
    let voice_idx = schema_voice_idx(params, state.selected_instrument);
    let layout_snapshot =
        PersistentField::<TrackLayoutState>::map(&params.track_layout, |s| s.clone());

    ui.set_width(ui.available_width());

    let header_rect = ui
        .allocate_exact_size(Vec2::new(ui.available_width(), 42.0), egui::Sense::hover())
        .0;
    ui.painter().rect_filled(header_rect, 0.0, PANEL);
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        egui::Stroke::new(1.0, LINE),
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(header_rect.shrink2(Vec2::new(14.0, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.horizontal_centered(|ui| {
                ui.set_height(header_rect.height());
                ui.label(
                    RichText::new("Sound Editor")
                        .font(f_sans_bold(13.0))
                        .color(Color32::WHITE),
                );
                ui.add_space(2.0);
                // Always show which lane is being edited (lanes are selected by
                // clicking them in the grid, there are no per-instrument tabs).
                let header_name = {
                    let slot_name = &layout_snapshot.slots[state.selected_instrument].name;
                    let name = if slot_name.is_empty() {
                        crate::instrument_registry::INSTRUMENTS[voice_idx].name
                    } else {
                        slot_name.as_str()
                    };
                    format!("Slot {} - {}", state.selected_instrument + 1, name)
                };
                ui.label(RichText::new(header_name).font(f_mono(11.0)).color(INK3));
                // (Engine selector belongs to the future modular phase — omitted for now.)
            });
        },
    );

    let tabs_rect = ui
        .allocate_exact_size(Vec2::new(ui.available_width(), 45.0), egui::Sense::hover())
        .0;
    ui.painter().rect_filled(tabs_rect, 0.0, PANEL);
    ui.painter().hline(
        tabs_rect.x_range(),
        tabs_rect.bottom(),
        egui::Stroke::new(1.0, LINE),
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(tabs_rect.shrink2(Vec2::new(12.0, 9.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP_TIGHT;
                // Two fixed tabs. The edited lane is chosen by clicking it in the
                // grid (auto-edit), not through per-instrument tab buttons.
                let tab_w = ((tabs_rect.width() - 24.0 - GAP_TIGHT) / 2.0).floor();
                for (tab, label, hover) in [
                    (
                        SoundEditorTab::Sound,
                        "Sound Editor",
                        "Synthesis settings of the selected lane",
                    ),
                    (
                        SoundEditorTab::Track,
                        "Track",
                        "Instrument type, MIDI note, routing, length",
                    ),
                ] {
                    let selected = state.sound_editor_tab == tab;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .monospace()
                            .size(10.5)
                            .color(if selected { Color32::WHITE } else { INK2 }),
                    )
                    .min_size(Vec2::new(tab_w, CTL_HEIGHT))
                    .fill(if selected { BLUE } else { PANEL2 })
                    .stroke(egui::Stroke::new(1.0, if selected { BLUE } else { LINE2 }))
                    .corner_radius(6.0);
                    if ui.add(btn).on_hover_text(hover).clicked() {
                        state.sound_editor_tab = tab;
                    }
                }
            });
        },
    );

    let inst = &sound_settings.instruments[state.selected_instrument];
    let (
        mut freq,
        mut decay,
        mut vol,
        mut filt,
        mut attack,
        mut release,
        mut decay_curve,
        mut release_curve,
        mut hold,
        mut filter_env_amount,
        mut filter_env_decay,
        mut analog,
        mut stereo,
    ) = inst.load();
    let mut changed = false;

    let scroll_height = ui.available_height().max(120.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(scroll_height)
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - 20.0);
            egui::Frame::new()
                .inner_margin(egui::Margin {
                    left: 14,
                    right: 14,
                    top: 6,
                    bottom: 14,
                })
                .show(ui, |ui| {
                    match state.sound_editor_tab {
                        SoundEditorTab::Sound => {

            // ------ Dev Tools: Preset Dumps ------
            if cfg!(debug_assertions) {
                ui.collapsing("Dev: Preset Dumps", |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut state.dump_name_input);
                if ui.button("Dump").clicked() {
                    let instrument = &crate::instrument_registry::INSTRUMENTS[voice_idx];
                    let mut specials = Vec::new();
                    for def in instrument.special_params {
                        specials.push(inst.special_value(def.special_index));
                    }
                    let algo = params.algos()[state.selected_instrument].value() as u8;
                    // Skip Analog for instruments that don't use it
                    let standards = if matches!(voice_idx, 2 | 3 | 7 | 8 | 10 | 12)
                    {
                        // HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap - use 0.0 as placeholder
                        [
                            freq,
                            decay,
                            vol,
                            filt,
                            attack,
                            release,
                            decay_curve,
                            release_curve,
                            hold,
                            filter_env_amount,
                            filter_env_decay,
                            0.0,
                            stereo,
                        ]
                    } else {
                        [
                            freq,
                            decay,
                            vol,
                            filt,
                            attack,
                            release,
                            decay_curve,
                            release_curve,
                            hold,
                            filter_env_amount,
                            filter_env_decay,
                            analog,
                            stereo,
                        ]
                    };

                    let dump = preset_dumps::PresetDump {
                        name: state.dump_name_input.clone(),
                        instrument_idx: state.selected_instrument,
                        instrument_label: instrument.label.to_string(),
                        standards,
                        algo,
                        specials,
                    };
                    if let Err(e) = preset_dumps::dump_preset(&dump) {
                        eprintln!("Dump failed: {}", e);
                    }
                }
            });
            let dumps = preset_dumps::list_dumps();
            if !dumps.is_empty() {
                ui.label("Existing dumps:");
                for info in dumps {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} - {}", info.instrument_label, info.name));
                        if ui.button("Load").clicked() {
                            if let Ok(dump) = preset_dumps::load_dump(&info.path) {
                                let dump_slot =
                                    dump.instrument_idx.min(crate::track::MAX_TRACKS - 1);
                                select_legacy_track(state, dump_slot);
                                let dump_voice = schema_voice_idx(params, dump_slot);
                                let target_inst = &sound_settings.instruments[dump_slot];
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::Freq,
                                    dump.standards[0],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::Decay,
                                    dump.standards[1],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::Volume,
                                    dump.standards[2],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::FilterFreq,
                                    dump.standards[3],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::Attack,
                                    dump.standards[4],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::Release,
                                    dump.standards[5],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::DecayCurve,
                                    dump.standards[6],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::ReleaseCurve,
                                    dump.standards[7],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::Hold,
                                    dump.standards[8],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::FilterEnvAmount,
                                    dump.standards[9],
                                );
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::FilterEnvDecay,
                                    dump.standards[10],
                                );
                                // Skip Analog for instruments that don't use it
                                let is_analog_fixed = matches!(
                                    dump_voice,
                                    2 | 3 | 7 | 8 | 10 | 12 // HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap
                                );
                                if !is_analog_fixed {
                                    store_field(
                                        target_inst,
                                        crate::instrument_registry::StandardField::Analog,
                                        dump.standards[11],
                                    );
                                }
                                store_field(
                                    target_inst,
                                    crate::instrument_registry::StandardField::Stereo,
                                    dump.standards[12],
                                );
                                let algo_param = params.algos()[dump_slot];
                                setter.set_parameter(algo_param, dump.algo as i32);
                                let inst_def =
                                    &crate::instrument_registry::INSTRUMENTS[dump_voice];
                                for (i, def) in inst_def.special_params.iter().enumerate() {
                                    if i < dump.specials.len() {
                                        target_inst
                                            .set_special(def.special_index, dump.specials[i]);
                                    }
                                }
                                sound_settings.bump_version();
                            }
                        }
                        if ui.button("Delete").clicked() {
                            let _ = preset_dumps::delete_dump(&info.path);
                        }
                    });
                }
            }
                });
            }
            ui.add(egui::Separator::default().spacing(8.0));

            // Volume en tête (sans titre de section) — même largeur que les sections.
            let vol_changed = ui
                .scope(|ui| {
                    ui.set_max_width(EDITOR_PARAMS_W);
                    draw_editor_slider_row(ui, "Volume", &mut vol, 0.0, 2.0, false, Some(""))
                        .changed()
                })
                .inner;
            if vol_changed {
                store_field(inst, crate::instrument_registry::StandardField::Volume, vol);
                changed = true;
            }
            ui.add(egui::Separator::default().spacing(6.0));

            // Data-driven grouped Sound Panel (schema follows the slot's kind)
            let instrument = &crate::instrument_registry::INSTRUMENTS[voice_idx];
            let standard_defs = instrument.standard_params;
            let special_defs = instrument.special_params;

            for family in [
                crate::instrument_registry::ParamFamily::Osc,
                crate::instrument_registry::ParamFamily::Env,
                crate::instrument_registry::ParamFamily::Analog,
                crate::instrument_registry::ParamFamily::Filter,
                crate::instrument_registry::ParamFamily::Saturation,
                crate::instrument_registry::ParamFamily::Output,
            ] {
                // Skip a section that has no parameters for this instrument
                // (e.g. Saturation on instruments without a saturation pack).
                let fam_has_std = standard_defs.iter().any(|d| {
                    d.family == family
                        && d.field != crate::instrument_registry::StandardField::Volume
                });
                let fam_has_special = special_defs.iter().any(|d| d.family == family);
                if !fam_has_std
                    && !fam_has_special
                    && family != crate::instrument_registry::ParamFamily::Output
                {
                    continue;
                }
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(2.0);
                let section_title = match family {
                    crate::instrument_registry::ParamFamily::Osc => "Oscillator",
                    crate::instrument_registry::ParamFamily::Env => "Envelope",
                    crate::instrument_registry::ParamFamily::Analog => "Analog",
                    crate::instrument_registry::ParamFamily::Filter => "Filter",
                    crate::instrument_registry::ParamFamily::Saturation => "Saturation",
                    crate::instrument_registry::ParamFamily::Output => "Output",
                };
                ui.label(RichText::new(section_title).font(f_sans_sb(10.5)).color(INK3));
                ui.add_space(6.0);

            let has_filter_env = standard_defs
                .iter()
                .any(|d| d.field == crate::instrument_registry::StandardField::FilterEnvAmount);
            let has_graph = family == crate::instrument_registry::ParamFamily::Env
                || (family == crate::instrument_registry::ParamFamily::Filter && has_filter_env);
            let params_w = EDITOR_PARAMS_W;
            ui.horizontal(|ui| {
                // Left column: params (width-constrained so the graph keeps its space)
                ui.vertical(|ui| {
                    ui.set_max_width(params_w);
                    ui.set_width(params_w);
                    ui.spacing_mut().item_spacing.y = 9.0;
                    // Standard params for this family
                    for def in standard_defs.iter().filter(|d| {
                        d.family == family
                            && d.field != crate::instrument_registry::StandardField::Volume
                    }) {
                            ui.horizontal(|ui| {
                                let label_text = if def.field == crate::instrument_registry::StandardField::FilterFreq {
                                    let ft = crate::instrument_registry::filter_type_label(voice_idx);
                                    format!("{} ({})", def.label, ft)
                                } else {
                                    def.label.to_string()
                                };

                                let is_bass_drum = voice_idx == 0 || voice_idx == 11;
                                let freq_in_notes = is_bass_drum
                                    && def.field == crate::instrument_registry::StandardField::Freq
                                    && inst.freq_mode();

                                     match (&def.widget, def.field) {
                                 (crate::instrument_registry::ParamWidget::Slider { min, max, logarithmic, suffix }, field) => {
                                    // Bass drums can display frequency as Hz or musical notes.
                                    if is_bass_drum && field == crate::instrument_registry::StandardField::Freq {
                                        let ratio = instrument.freq_display_ratio;
                                        let row = draw_editor_frequency_row(
                                            ui,
                                            ("freq_mode", state.selected_instrument),
                                            &label_text,
                                            &mut freq,
                                            *min,
                                            *max,
                                            *logarithmic,
                                            ratio,
                                            freq_in_notes,
                                        );
                                        if row.value_changed || row.response.changed() {
                                            store_field(inst, field, freq);
                                            changed = true;
                                        }
                                        if let Some(new_mode) = row.mode_change {
                                            inst.set_freq_mode(new_mode);
                                            sound_settings.bump_version();
                                            if new_mode {
                                                let snapped_note = freq_to_note(freq * ratio).round();
                                                freq = note_to_freq(snapped_note) / ratio;
                                                store_field(inst, field, freq);
                                                changed = true;
                                            }
                                        }
                                    } else {
                                        let value: &mut f32 = match field {
                                            crate::instrument_registry::StandardField::Freq => &mut freq,
                                            crate::instrument_registry::StandardField::Decay => &mut decay,
                                            crate::instrument_registry::StandardField::Volume => &mut vol,
                                            crate::instrument_registry::StandardField::FilterFreq => &mut filt,
                                            crate::instrument_registry::StandardField::Attack => &mut attack,
                                            crate::instrument_registry::StandardField::Release => &mut release,
                                            crate::instrument_registry::StandardField::DecayCurve => &mut decay_curve,
                                            crate::instrument_registry::StandardField::ReleaseCurve => &mut release_curve,
                                            crate::instrument_registry::StandardField::Hold => &mut hold,
                                            crate::instrument_registry::StandardField::FilterEnvAmount => &mut filter_env_amount,
                                            crate::instrument_registry::StandardField::FilterEnvDecay => &mut filter_env_decay,
                                            crate::instrument_registry::StandardField::Analog => &mut analog,
                                            crate::instrument_registry::StandardField::Stereo => &mut stereo,
                                        };
                                        if draw_editor_slider_row(
                                            ui,
                                            &label_text,
                                            value,
                                            *min,
                                            *max,
                                            *logarithmic,
                                            *suffix,
                                        )
                                        .changed()
                                        {
                                            store_field(inst, field, *value);
                                            changed = true;
                                        }
                                    }
                                }
                                (crate::instrument_registry::ParamWidget::Checkbox, field) => {
                                    let value: &mut f32 = match field {
                                        crate::instrument_registry::StandardField::Stereo => &mut stereo,
                                        _ => &mut stereo,
                                    };
                                    if draw_editor_switch_row(ui, &label_text, value).changed() {
                                        store_field(inst, field, *value);
                                        changed = true;
                                    }
                                }
                            }
                        });
                    }

                    // Special params for this family — stored PER SLOT so two
                    // slots of the same kind stay independent.
                    for def in special_defs.iter().filter(|d| d.family == family) {
                        ui.horizontal(|ui| {
                            let current = inst.special_value(def.special_index);
                            let mut new_value = None;
                            // Boolean toggle for on/off switches (min=0, max=1)
                            if def.min == 0.0 && def.max == 1.0 && def.label.to_lowercase().contains("pre-filter") {
                                let mut value = current;
                                if draw_editor_switch_row(ui, def.label, &mut value).changed() {
                                    new_value = Some(value);
                                }
                            // Saturation Type: show select with names instead of number slider
                            } else if def.label.to_lowercase().contains("saturation type") {
                                let type_names = ["None", "SoftClip", "Valve", "Transistor", "HardClip", "Tape"];
                                let current_idx = (current as usize).min(type_names.len().saturating_sub(1));
                                editor_label(ui, def.label);
                                if let (_, Some(idx)) = styled_select(ui, def.name, current_idx, &type_names, 146.0) {
                                    new_value = Some(idx as f32);
                                }
                            // Cymbal Noise Type: show select with names
                            } else if def.label.to_lowercase().contains("noise type") {
                                let type_names = ["White", "Pink", "Brown", "Blue"];
                                let current_idx = (current as usize).min(type_names.len().saturating_sub(1));
                                editor_label(ui, def.label);
                                if let (_, Some(idx)) = styled_select(ui, def.name, current_idx, &type_names, 146.0) {
                                    new_value = Some(idx as f32);
                                }
                            // Kick Click Type: show select with names
                            } else if def.label.to_lowercase().contains("click type") {
                                let type_names = ["Soft", "Medium", "Hard"];
                                let current_idx = (current as usize).min(type_names.len().saturating_sub(1));
                                editor_label(ui, def.label);
                                if let (_, Some(idx)) = styled_select(ui, def.name, current_idx, &type_names, 146.0) {
                                    new_value = Some(idx as f32);
                                }
                            } else {
                                let mut value = current;
                                let logarithmic = def.min > 0.0 && def.max / def.min >= 20.0;
                                if draw_editor_slider_row(
                                    ui,
                                    def.label,
                                    &mut value,
                                    def.min,
                                    def.max,
                                    logarithmic,
                                    None,
                                )
                                .changed()
                                {
                                    new_value = Some(value);
                                }
                            }
                            if let Some(value) = new_value {
                                inst.set_special(def.special_index, value);
                                sound_settings.bump_version();
                            }
                        });
                    }

                    // Algorithm selector inside OSC family
                    if family == crate::instrument_registry::ParamFamily::Osc {
                        if let Some(voice) = DrumVoice::from_index(voice_idx) {
                            let algos = synthesis::algos_for(voice);
                            if algos.len() > 1 && voice_idx != 3 {
                                let algo_param = params.algos()[state.selected_instrument];
                                ui.horizontal(|ui| {
                                    editor_label(ui, "Algorithm");
                                    let algo_names: Vec<&str> = algos.iter().map(|a| a.name).collect();
                                    algo_combo(ui, setter, algo_param, &algo_names);
                                });
                            }
                        }
                    }

                    // Mix checkbox inside OUTPUT family
                    if family == crate::instrument_registry::ParamFamily::Output {
                        let mix_param = params.mixes()[state.selected_instrument];
                        let mut mixf = if mix_param.value() { 1.0 } else { 0.0 };
                        if draw_editor_switch_row(ui, "Mix", &mut mixf).changed() {
                            setter.set_parameter(mix_param.into(), mixf >= 0.5);
                        }
                    }

                });

                // Right column: graphs (gap so they aren't cramped against the params)
                if has_graph {
                    ui.add_space(16.0);
                }
                match family {
                    crate::instrument_registry::ParamFamily::Env => {
                        let has_release = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::Release);
                        draw_amp_envelope(ui, attack, decay, decay_curve, hold, release, release_curve, has_release);
                    }
                    crate::instrument_registry::ParamFamily::Filter => {
                        let has_filter_env = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::FilterEnvAmount);
                        if has_filter_env {
                            draw_filter_envelope(ui, decay_curve, filter_env_decay);
                        }
                    }
                    _ => {}
                }
            });
            }
                    }
                        SoundEditorTab::Track => {
                            draw_track_tab(ui, params, sound_settings, setter, state);
                        }
                    }
                });
        });

    if changed {
        sound_settings.bump_version();
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------------------------
fn store_field(
    inst: &crate::sound_settings::InstrumentSettingsState,
    field: crate::instrument_registry::StandardField,
    value: f32,
) {
    use crate::instrument_registry::StandardField;
    match field {
        StandardField::Freq => inst.frequency.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Decay => inst.decay.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Volume => inst.volume.store(value.to_bits(), Ordering::Relaxed),
        StandardField::FilterFreq => inst.filter_freq.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Attack => inst.attack.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Release => inst.release.store(value.to_bits(), Ordering::Relaxed),
        StandardField::DecayCurve => inst.decay_curve.store(value.to_bits(), Ordering::Relaxed),
        StandardField::ReleaseCurve => inst.release_curve.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Hold => inst.hold.store(value.to_bits(), Ordering::Relaxed),
        StandardField::FilterEnvAmount => inst
            .filter_env_amount
            .store(value.to_bits(), Ordering::Relaxed),
        StandardField::FilterEnvDecay => inst
            .filter_env_decay
            .store(value.to_bits(), Ordering::Relaxed),
        StandardField::Analog => inst.analog.store(value.to_bits(), Ordering::Relaxed),
        StandardField::Stereo => inst.stereo.store(value.to_bits(), Ordering::Relaxed),
    }
}

fn load_pattern_for_ui(pattern_for_ui: &SharedPattern, pattern: &Pattern) {
    let masks = pattern.step_masks();
    pattern_for_ui.load_step_masks(&masks);
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        pattern_for_ui.store_fusions(inst, &pattern.fusions[inst]);
    }
}

fn load_pattern_for_ui_with_length(
    pattern_for_ui: &SharedPattern,
    pattern: &Pattern,
    length: usize,
) {
    let mut masks = pattern.step_masks();
    // Clear steps beyond the current pattern length
    for step in length..masks.len() {
        masks[step] = 0;
    }
    pattern_for_ui.load_step_masks(&masks);
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        pattern_for_ui.store_fusions(inst, &pattern.fusions[inst]);
    }
}

fn toggle_step_for_ui(pattern_for_ui: &SharedPattern, step: usize, instrument: usize) {
    let current_mask = pattern_for_ui.load_step_mask(step);
    let bit = 1u16 << instrument;
    let next_mask = current_mask ^ bit;
    pattern_for_ui.set_step_mask(step, next_mask);
}

fn set_step_active_for_ui(
    pattern_for_ui: &SharedPattern,
    step: usize,
    instrument: usize,
    active: bool,
) {
    let current_mask = pattern_for_ui.load_step_mask(step);
    let bit = 1u16 << instrument;
    let next_mask = if active {
        current_mask | bit
    } else {
        current_mask & !bit
    };
    pattern_for_ui.set_step_mask(step, next_mask);
}

fn fusion_containing(fusions: &[FusedGroup], step: usize) -> Option<(usize, FusedGroup)> {
    fusions
        .iter()
        .copied()
        .enumerate()
        .find(|(_, group)| group.contains(step))
}

fn reset_stutter_on_fusion(
    params: &DrumFlashParams,
    instrument: usize,
    start_cell: usize,
    end_cell: usize,
) {
    let seq_plock = &params.seq_plock_state.state;
    for step in start_cell..=end_cell {
        if let Some(mut seq_params) = seq_plock.get(instrument, step) {
            if seq_params.stutter_count != 1 {
                seq_params.stutter_count = 1;
                seq_plock.set(instrument, step, &seq_params);
            }
        }
    }
}

fn clear_covered_plocks_for_fusion(
    plock: &PlockState,
    params: &DrumFlashParams,
    instrument: usize,
    start_cell: usize,
    end_cell: usize,
) {
    // The fusion start cell is the only source for sound/seq plocks. Plocks on
    // covered cells would be invisible and inactive while the fusion exists.
    for step in (start_cell + 1)..=end_cell {
        plock.clear(instrument, step);
        params.seq_plock_state.state.clear(instrument, step);
    }
}

fn normalize_fusion_cells_for_ui(
    pattern_for_ui: &SharedPattern,
    instrument: usize,
    start_cell: usize,
    end_cell: usize,
) {
    for step in start_cell..=end_cell {
        set_step_active_for_ui(pattern_for_ui, step, instrument, false);
    }
    set_step_active_for_ui(pattern_for_ui, start_cell, instrument, true);
}

fn toggle_fusion_for_ui(pattern_for_ui: &SharedPattern, group: FusedGroup, instrument: usize) {
    let start = group.start_cell as usize;
    let end = group.end_cell as usize;
    let next_active = !pattern_for_ui.is_active(start, instrument);
    for step in start..=end {
        set_step_active_for_ui(pattern_for_ui, step, instrument, false);
    }
    set_step_active_for_ui(pattern_for_ui, start, instrument, next_active);
}

fn edited_fusion_for_ui(
    pattern_for_ui: &SharedPattern,
    state: &EditorUIState,
) -> Option<(usize, usize, FusedGroup)> {
    let (instrument, index) = state.fusion_editing?;
    pattern_for_ui
        .load_fusions(instrument)
        .get(index)
        .copied()
        .map(|group| (instrument, index, group))
}

fn finish_fusion_editing_for_ui(pattern_for_ui: &SharedPattern, state: &mut EditorUIState) {
    if let Some((instrument, _, group)) = edited_fusion_for_ui(pattern_for_ui, state) {
        set_step_active_for_ui(pattern_for_ui, group.start_cell as usize, instrument, true);
    }
    state.fusion_editing = None;
}

fn draw_fusion_idle_box_contents(ui: &mut egui::Ui, fusion_mode_active: bool) {
    ui.label(RichText::new("Fusion").strong().size(11.0));
    ui.separator();

    if fusion_mode_active {
        ui.label(
            RichText::new("Select 2 cells")
                .strong()
                .size(11.0)
                .color(BLUE),
        );
    } else {
        ui.label(RichText::new("Maj for fusion mode").size(11.0).color(INK2));
    }
}

fn current_field_value_for_fusion(
    _params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    instrument: usize,
    field_index: usize,
) -> f32 {
    match field_index {
        0..=11 | 18 => {
            let inst = &sound_settings.instruments[instrument];
            let (freq, decay, vol, filt, attack, release, dc, rc, hold, fea, fed, analog, stereo) =
                inst.load();
            match field_index {
                0 => freq,
                1 => decay,
                2 => vol,
                3 => filt,
                4 => release,
                5 => dc,
                6 => rc,
                7 => hold,
                8 => fea,
                9 => fed,
                10 => analog,
                11 => stereo,
                18 => attack,
                _ => 0.0,
            }
        }
        _ => {
            if field_index >= crate::plock::SPECIAL_FIELD_START {
                let special_index = field_index - crate::plock::SPECIAL_FIELD_START;
                sound_settings.instruments[instrument].special_value(special_index)
            } else {
                0.0
            }
        }
    }
}

/// Read the current end value for a morph target, or the global/plock value if
/// the field is not a target. Uses the live `new_fusions` slice so mutations
/// made earlier in the same frame are visible.
fn fusion_morph_state(
    new_fusions: &[crate::sequencer::pattern::FusedGroup],
    fusion_index: usize,
    field_index: usize,
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    instrument: usize,
) -> (f32, bool) {
    let Some(group) = new_fusions.get(fusion_index) else {
        return (0.0, false);
    };
    if group.has_morph_target(field_index) {
        let end = group.morph_targets[..group.morph_count as usize]
            .iter()
            .find(|t| t.field == field_index as u8)
            .map(|t| t.end_value)
            .unwrap_or(0.0);
        (end, true)
    } else {
        (
            current_field_value_for_fusion(params, sound_settings, instrument, field_index),
            false,
        )
    }
}

fn draw_fusion_edit_box(
    ui: &mut egui::Ui,
    pattern_for_ui: &SharedPattern,
    params: &DrumFlashParams,
    _sound_settings: &SoundSettingsState,
    state: &mut EditorUIState,
    fusion_mode_active: bool,
) -> egui::Rect {
    let box_size = Vec2::new(380.0, 28.0);

    // Allocate the exact outer size so the parent row never grows, even if the
    // edit-content widgets are slightly taller than the idle-content widgets.
    let (rect, response) = ui.allocate_exact_size(box_size, egui::Sense::hover());
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(24, 24, 30))
                .stroke(egui::Stroke::new(1.0, LINE2))
                .corner_radius(5.0)
                .inner_margin(3.0)
                .show(ui, |ui| {
                    let inner_size = Vec2::new(box_size.x - 6.0, box_size.y - 6.0);
                    ui.set_min_size(inner_size);
                    ui.set_max_size(inner_size);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        if let Some((instrument, index, group)) =
                            edited_fusion_for_ui(pattern_for_ui, state)
                        {
                            ui.label(
                                RichText::new(format!(
                                    "F {}-{}",
                                    group.start_cell + 1,
                                    group.end_cell + 1
                                ))
                                .strong()
                                .size(11.0),
                            );
                            ui.label(RichText::new("Steps:").size(11.0));

                            let step_drag = egui::DragValue::new(&mut state.fusion_edit_steps)
                                .range(1..=64)
                                .speed(1.0)
                                .fixed_decimals(0);
                            let step_response = ui.add_sized(Vec2::new(40.0, 18.0), step_drag);
                            if state.fusion_edit_focus_request {
                                state.fusion_edit_focus_request = false;
                                step_response.request_focus();
                            }
                            if step_response.lost_focus() {
                                let mut new_fusions = pattern_for_ui.load_fusions(instrument);
                                if let Some(group) = new_fusions.get_mut(index) {
                                    group.step_count = state.fusion_edit_steps;
                                    pattern_for_ui.store_fusions(instrument, &new_fusions);
                                }
                                finish_fusion_editing_for_ui(pattern_for_ui, state);
                            }

                            // Morph targets display (compact)
                            if group.morph_count > 0 {
                                let morphable = crate::instrument_registry::morphable_fields(
                                    schema_voice_idx(params, instrument),
                                );
                                let names: Vec<&str> = group.morph_targets
                                    [..group.morph_count as usize]
                                    .iter()
                                    .map(|t| {
                                        morphable
                                            .iter()
                                            .find(|f| f.field_index == t.field as usize)
                                            .map(|f| f.label)
                                            .unwrap_or("?")
                                    })
                                    .collect();
                                ui.label(
                                    RichText::new(format!("M: {}", names.join(", ")))
                                        .size(10.0)
                                        .color(INK2),
                                );
                            } else {
                                ui.label(RichText::new("M: Off").size(10.0).color(INK2));
                            }

                            let del_clicked = ui
                                .allocate_ui_with_layout(
                                    Vec2::new(32.0, 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| ui.add(egui::Button::new("Del").small()),
                                )
                                .inner
                                .clicked();
                            if del_clicked {
                                let mut new_fusions = pattern_for_ui.load_fusions(instrument);
                                if index < new_fusions.len() {
                                    new_fusions.remove(index);
                                    pattern_for_ui.store_fusions(instrument, &new_fusions);
                                }
                                state.fusion_editing = None;
                            }
                            let close_clicked = ui
                                .allocate_ui_with_layout(
                                    Vec2::new(22.0, 18.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| ui.add(egui::Button::new("×").small()),
                                )
                                .inner
                                .clicked();
                            if close_clicked {
                                finish_fusion_editing_for_ui(pattern_for_ui, state);
                            }
                        } else {
                            if state.fusion_editing.is_some() {
                                state.fusion_editing = None;
                            }
                            draw_fusion_idle_box_contents(ui, fusion_mode_active);
                        }
                    });
                });
        },
    );

    response.rect
}

fn close_fusion_editing_on_outside_click(
    ui: &egui::Ui,
    pattern_for_ui: &SharedPattern,
    state: &mut EditorUIState,
    inline_rect: Option<egui::Rect>,
    edit_box_rect: Option<egui::Rect>,
) {
    if state.fusion_editing.is_none() {
        return;
    }

    let clicked_outside = ui.input(|input| {
        if !input.pointer.any_pressed() {
            return false;
        }
        let Some(pos) = input.pointer.interact_pos() else {
            return false;
        };

        let inside_inline = inline_rect.map(|rect| rect.contains(pos)).unwrap_or(false);
        let inside_box = edit_box_rect
            .map(|rect| rect.contains(pos))
            .unwrap_or(false);

        !inside_inline && !inside_box
    });

    if clicked_outside {
        finish_fusion_editing_for_ui(pattern_for_ui, state);
    }
}

fn fusion_inside_range(group: FusedGroup, start: usize, end: usize) -> bool {
    (group.start_cell as usize) >= start && (group.end_cell as usize) < end
}

fn fusion_overlaps_range(group: FusedGroup, start: usize, end: usize) -> bool {
    (group.start_cell as usize) < end && (group.end_cell as usize) >= start
}

fn normalize_existing_fusion_cells_for_ui(
    pattern_for_ui: &SharedPattern,
    instrument: usize,
    start_cell: usize,
    end_cell: usize,
) {
    let was_active = pattern_for_ui.is_active(start_cell, instrument);
    for step in start_cell..=end_cell {
        set_step_active_for_ui(pattern_for_ui, step, instrument, false);
    }
    set_step_active_for_ui(pattern_for_ui, start_cell, instrument, was_active);
}

#[allow(dead_code)] // retained: per-page Copy/Paste/Clear menu, re-wired in the Page-bar phase
fn clear_page_fusions_for_ui(pattern_for_ui: &SharedPattern, page: usize) {
    let page_start = page * 16;
    let page_end = (page_start + 16).min(crate::sequencer::pattern::STEP_COUNT);
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        let fusions = pattern_for_ui.load_fusions(inst);
        let retained: Vec<_> = fusions
            .into_iter()
            .filter(|group| !fusion_inside_range(*group, page_start, page_end))
            .collect();
        pattern_for_ui.store_fusions(inst, &retained);
    }
}

fn copy_page_to_clipboard(
    pattern_for_ui: &SharedPattern,
    plock: &PlockState,
    params: &DrumFlashParams,
    page: usize,
) -> PageClipboard {
    let page_start = page * 16;
    let page_end = (page_start + 16).min(crate::sequencer::pattern::STEP_COUNT);
    let mut triggers = [0u16; 16];
    for (i, step) in (page_start..page_end).enumerate() {
        triggers[i] = pattern_for_ui.load_step_mask(step);
    }

    let mut plocks = Vec::new();
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        for (i, step) in (page_start..page_end).enumerate() {
            if plock.masks.is_active(inst, step) {
                let field_mask = plock.field_masks.get_raw(inst, step);
                let mut values = vec![0.0f32; crate::plock::FIELD_COUNT];
                for field in 0..crate::plock::FIELD_COUNT {
                    values[field] = plock.values.get(inst, step, field);
                }
                plocks.push(PlockClipboardEntry {
                    instrument: inst,
                    step: i,
                    field_mask,
                    values,
                });
            }
            let seq_plock = &params.seq_plock_state.state;
            if seq_plock.is_active(inst, step) {
                // Seq plocks are stored per step in PageClipboard via an extension field.
                // For now, sound plocks only. Seq plocks will be added if needed.
                let _ = seq_plock;
            }
        }
    }

    let mut fusions = Vec::new();
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        for group in pattern_for_ui.load_fusions(inst) {
            let group_start = group.start_cell as usize;
            let group_end = group.end_cell as usize;
            if group_start >= page_start && group_end < page_end {
                fusions.push(FusionClipboardEntry {
                    instrument: inst,
                    start_step: group_start - page_start,
                    end_step: group_end - page_start,
                    step_count: group.step_count,
                });
            }
        }
    }

    PageClipboard {
        triggers,
        plocks,
        fusions,
    }
}

fn paste_page_from_clipboard(
    pattern_for_ui: &SharedPattern,
    plock: &PlockState,
    params: &DrumFlashParams,
    page: usize,
    clipboard: &PageClipboard,
) {
    let page_start = page * 16;
    let page_end = (page_start + 16).min(crate::sequencer::pattern::STEP_COUNT);

    // Triggers
    for (i, step) in (page_start..page_end).enumerate() {
        pattern_for_ui.set_step_mask(step, clipboard.triggers[i]);
    }

    // Clear existing plocks on the page
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        for step in page_start..page_end {
            plock.clear(inst, step);
            params.seq_plock_state.state.clear(inst, step);
        }
    }

    // Sound plocks
    for entry in &clipboard.plocks {
        let step = page_start + entry.step;
        if step >= page_end {
            continue;
        }
        plock.masks.set_active(entry.instrument, step, true);
        plock
            .field_masks
            .set_raw(entry.instrument, step, entry.field_mask);
        for (field, &value) in entry.values.iter().enumerate() {
            plock.values.set(entry.instrument, step, field, value);
        }
    }

    // Fusions
    replace_page_fusions_for_ui(pattern_for_ui, params, plock, page, &clipboard.fusions);
}

fn clear_page_for_ui(
    pattern_for_ui: &SharedPattern,
    plock: &PlockState,
    params: &DrumFlashParams,
    page: usize,
) {
    let page_start = page * 16;
    let page_end = (page_start + 16).min(crate::sequencer::pattern::STEP_COUNT);

    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        for step in page_start..page_end {
            set_step_active_for_ui(pattern_for_ui, step, inst, false);
            plock.clear(inst, step);
            params.seq_plock_state.state.clear(inst, step);
        }
    }

    clear_page_fusions_for_ui(pattern_for_ui, page);
}

#[allow(dead_code)] // retained: per-page Copy/Paste/Clear menu, re-wired in the Page-bar phase
fn replace_page_fusions_for_ui(
    pattern_for_ui: &SharedPattern,
    params: &DrumFlashParams,
    plock: &PlockState,
    page: usize,
    entries: &[FusionClipboardEntry],
) {
    let page_start = page * 16;
    let page_end = (page_start + 16).min(crate::sequencer::pattern::STEP_COUNT);

    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        let mut new_fusions: Vec<_> = pattern_for_ui
            .load_fusions(inst)
            .into_iter()
            .filter(|group| !fusion_inside_range(*group, page_start, page_end))
            .collect();

        for entry in entries.iter().filter(|entry| entry.instrument == inst) {
            if entry.start_step >= entry.end_step || entry.end_step >= 16 {
                continue;
            }

            let start_cell = page_start + entry.start_step;
            let end_cell = page_start + entry.end_step;
            let group = FusedGroup {
                start_cell: start_cell as u8,
                end_cell: end_cell as u8,
                step_count: entry.step_count,
                ..Default::default()
            };
            if !group.is_valid() {
                continue;
            }

            normalize_existing_fusion_cells_for_ui(pattern_for_ui, inst, start_cell, end_cell);
            reset_stutter_on_fusion(params, inst, start_cell, end_cell);
            clear_covered_plocks_for_fusion(plock, params, inst, start_cell, end_cell);
            new_fusions.push(group);
        }

        new_fusions.sort_by_key(|group| group.start_cell);
        pattern_for_ui.store_fusions(inst, &new_fusions);
    }
}

fn duplicate_fusions_for_x2(
    pattern_for_ui: &SharedPattern,
    params: &DrumFlashParams,
    plock: &PlockState,
    current_len: usize,
) {
    let source_start = 0;
    let source_end = current_len.min(crate::sequencer::pattern::STEP_COUNT);
    let destination_start = current_len;
    let destination_end = (current_len * 2).min(crate::sequencer::pattern::STEP_COUNT);
    if source_end == 0 || destination_start >= destination_end {
        return;
    }

    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        let existing = pattern_for_ui.load_fusions(inst);
        let mut new_fusions: Vec<_> = existing
            .iter()
            .copied()
            .filter(|group| !fusion_overlaps_range(*group, destination_start, destination_end))
            .collect();

        for group in existing
            .iter()
            .copied()
            .filter(|group| fusion_inside_range(*group, source_start, source_end))
        {
            let start_cell = group.start_cell as usize + current_len;
            let end_cell = group.end_cell as usize + current_len;
            if end_cell >= destination_end {
                continue;
            }

            let shifted = FusedGroup {
                start_cell: start_cell as u8,
                end_cell: end_cell as u8,
                step_count: group.step_count,
                ..Default::default()
            };
            if !shifted.is_valid() {
                continue;
            }

            normalize_existing_fusion_cells_for_ui(pattern_for_ui, inst, start_cell, end_cell);
            reset_stutter_on_fusion(params, inst, start_cell, end_cell);
            clear_covered_plocks_for_fusion(plock, params, inst, start_cell, end_cell);
            new_fusions.push(shifted);
        }

        new_fusions.sort_by_key(|group| group.start_cell);
        pattern_for_ui.store_fusions(inst, &new_fusions);
    }
}

fn handle_fusion_shift_click(
    pattern_for_ui: &SharedPattern,
    params: &DrumFlashParams,
    plock: &PlockState,
    instrument: usize,
    clicked_step: usize,
    master_length: usize,
    fusions: &[FusedGroup],
    selection_start: &mut Option<usize>,
) {
    if let Some(start) = *selection_start {
        let (start_cell, end_cell) = if start < clicked_step {
            (start, clicked_step)
        } else {
            (clicked_step, start)
        };
        let span = end_cell - start_cell + 1;
        let same_page = start_cell / 16 == end_cell / 16;

        if span >= 2 && same_page && end_cell < master_length {
            normalize_fusion_cells_for_ui(pattern_for_ui, instrument, start_cell, end_cell);
            reset_stutter_on_fusion(params, instrument, start_cell, end_cell);
            clear_covered_plocks_for_fusion(plock, params, instrument, start_cell, end_cell);

            let mut new_fusions: Vec<_> = fusions
                .iter()
                .copied()
                .filter(|g| g.end_cell < start_cell as u8 || g.start_cell > end_cell as u8)
                .collect();
            new_fusions.push(FusedGroup {
                start_cell: start_cell as u8,
                end_cell: end_cell as u8,
                step_count: span.min(64) as u8,
                ..Default::default()
            });
            new_fusions.sort_by_key(|g| g.start_cell);
            pattern_for_ui.store_fusions(instrument, &new_fusions);
        }
        *selection_start = None;
    } else if clicked_step < master_length {
        *selection_start = Some(clicked_step);
    }
}

/// Clear all fusions for all instruments (used by Clear, presets, generator).
fn clear_all_fusions(pattern: &SharedPattern) {
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        pattern.store_fusions(inst, &[]);
    }
}

struct MixerRow<'a> {
    mute: &'a BoolParam,
    solo: &'a BoolParam,
}

fn mixer_rows(params: &DrumFlashParams) -> [MixerRow<'_>; crate::track::MAX_TRACKS] {
    std::array::from_fn(|i| MixerRow {
        mute: params.mutes()[i],
        solo: params.solos()[i],
    })
}

fn toggle_led_param(ui: &mut egui::Ui, setter: &ParamSetter, param: &BoolParam, label: &str) {
    let value = param.value();
    if ui.add(ToggleLED::new(label, value)).clicked() {
        let new_value = !value;
        setter.begin_set_parameter(param);
        setter.set_parameter(param, new_value);
        setter.end_set_parameter(param);
    }
}

fn enum_combo<E: nih_plug::prelude::Enum + PartialEq + 'static>(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &EnumParam<E>,
    label: &str,
) {
    let current = param.value();
    let current_idx = current.to_index();
    let variants = E::variants();
    if !label.is_empty() {
        ui.label(RichText::new(label).font(f_sans_sb(10.5)).color(INK3));
    }
    if let (_, Some(i)) = styled_select(ui, ("enum_combo", label), current_idx, variants, 116.0) {
        if i != current_idx {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, E::from_index(i));
            setter.end_set_parameter(param);
        }
    }
}

fn enum_combo_compact<E: nih_plug::prelude::Enum + PartialEq + 'static>(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &EnumParam<E>,
    id: &'static str,
    width: f32,
) {
    let current = param.value();
    let current_idx = current.to_index();
    let variants = E::variants();
    if let (_, Some(i)) = styled_select(ui, id, current_idx, variants, width) {
        if i != current_idx {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, E::from_index(i));
            setter.end_set_parameter(param);
        }
    }
}

fn compact_chip(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    compact_chip_colored(ui, label, active, BLUE)
}

fn compact_chip_colored(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    accent: Color32,
) -> egui::Response {
    let text_color = if active { Color32::WHITE } else { INK2 };
    let fill = if active { accent } else { PANEL2 };
    let stroke = if active { accent } else { LINE2 };
    ui.add(
        egui::Button::new(RichText::new(label).size(10.5).color(text_color))
            .min_size(Vec2::new(42.0, CTL_HEIGHT))
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(6.0),
    )
}

fn genrow_label(ui: &mut egui::Ui, label: &str, min_w: f32) {
    ui.add_sized(
        Vec2::new(min_w, CTL_HEIGHT),
        egui::Label::new(RichText::new(label).font(f_mono_med(9.5)).color(INK3)),
    );
}

fn chip_button(
    ui: &mut egui::Ui,
    label: &str,
    accent: bool,
    color: Color32,
    sense: egui::Sense,
) -> egui::Response {
    let text_color = if accent { color } else { INK2 };
    let stroke = if accent { color } else { LINE2 };
    let fill = if accent {
        Color32::from_rgba_premultiplied(
            ((color.r() as f32) * 0.12) as u8,
            ((color.g() as f32) * 0.12) as u8,
            ((color.b() as f32) * 0.12) as u8,
            255,
        )
    } else {
        PANEL2
    };
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(10.5)
                .color(text_color)
                .font(f_sans_sb(11.0)),
        )
        .min_size(Vec2::new(0.0, CTL_HEIGHT))
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(6.0)
        .sense(sense),
    )
}

fn algo_combo(ui: &mut egui::Ui, setter: &ParamSetter, param: &IntParam, algo_names: &[&str]) {
    let current = param.value() as usize;
    let current_clamped = current.min(algo_names.len().saturating_sub(1));
    if let (_, Some(i)) = styled_select(ui, "algo_combo", current_clamped, algo_names, 146.0) {
        if i != current_clamped {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, i as i32);
            setter.end_set_parameter(param);
        }
    }
}

fn export_midi_to_documents(
    pattern: &SharedPattern,
    track_layout: &crate::track::AtomicTrackLayout,
    bpm: f32,
    pattern_length: usize,
    swing: f32,
    groove_type: crate::groove::GrooveType,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let docs = std::env::var("USERPROFILE")
        .ok()
        .map(PathBuf::from)
        .map(|p| p.join("Documents"))
        .ok_or("Cannot find Documents folder")?;
    let export_dir = docs.join("Flash Drum").join("exports");
    create_dir_all(&export_dir)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let filename = format!("drum_pattern_{:.0}bpm_{}.mid", bpm, timestamp);
    let path = export_dir.join(filename);

    midi_export::export_pattern_to_midi(
        pattern,
        track_layout,
        bpm,
        pattern_length,
        swing,
        groove_type,
        &path,
    )?;
    Ok(path)
}

#[cfg(target_os = "windows")]
fn start_external_midi_drag(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let helper = find_midi_drag_helper().ok_or("MIDI drag helper not found")?;
    Command::new(helper).arg(path).spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn find_midi_drag_helper() -> Option<PathBuf> {
    const HELPER_NAME: &str = "drum-pattern-midi-drag-helper.exe";

    if let Ok(path) = std::env::var("DRUM_FLASH_MIDI_DRAG_HELPER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(common_files) = std::env::var("CommonProgramFiles") {
        let path = PathBuf::from(common_files)
            .join("VST3")
            .join("drum-pattern-vst.vst3")
            .join("Contents")
            .join("x86_64-win")
            .join(HELPER_NAME);
        if path.is_file() {
            return Some(path);
        }
    }

    let local_bundle = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("build")
        .join("drum-pattern-vst.vst3")
        .join("Contents")
        .join("x86_64-win")
        .join(HELPER_NAME);
    if local_bundle.is_file() {
        return Some(local_bundle);
    }

    None
}

#[cfg(not(target_os = "windows"))]
fn start_external_midi_drag(_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    Err("MIDI drag helper is only implemented on Windows".into())
}

// ---------------------------------------------------------------------------------------------------------------
// Plock context menu
// ---------------------------------------------------------------------------------------------------------------
fn draw_plock_menu(
    ui: &mut egui::Ui,
    plock: &PlockState,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    step: usize,
    state: &mut EditorUIState,
) {
    use crate::plock::{FIELD_COUNT, SPECIAL_FIELD_START};

    const ACCENT: Color32 = PL_LINK;
    // `instrument` is a SLOT index (plock storage is per slot); registry and
    // special-param lookups go through the voice index of the slot's kind.
    let voice_idx = schema_voice_idx(params, instrument);
    let inst_def = &crate::instrument_registry::INSTRUMENTS[voice_idx];
    let title = format!("Plock {}", inst_def.label);

    plock_menu_frame(ui, ACCENT, |ui| {
        if plock_menu_header(ui, &title, step, ACCENT) {
            state.plock_popup = None;
        }

        let inst = &sound_settings.instruments[instrument];
        let global = inst.load();
        let has_plock = plock.masks.is_active(instrument, step);

        // ------ Creation ------
        if !has_plock {
            ui.label(
                RichText::new("Create Plock")
                    .font(f_sans_sb(10.0))
                    .color(INK2),
            );
            ui.add_space(6.0);
            if plock_menu_action_row(ui, "Link to Global", ACCENT).clicked() {
                plock.masks.set_active(instrument, step, true);
            }
            if plock_menu_action_row(ui, "Snapshot Current Settings", ACCENT).clicked() {
                // Specials are stored per slot alongside the standard settings.
                let special = inst.load_specials();
                let algo = params.algos()[instrument].value() as u8;
                let settings = VoiceSettings {
                    frequency: global.0,
                    decay: global.1,
                    volume: global.2,
                    filter_freq: global.3,
                    attack: global.4,
                    release: global.5,
                    decay_curve: global.6,
                    release_curve: global.7,
                    hold: global.8,
                    filter_env_amount: global.9,
                    filter_env_decay: global.10,
                    analog: global.11,
                    stereo: global.12,
                    algo,
                    special,
                };
                plock.set_settings(instrument, step, &settings);
            }
            if let Some(ref entry) = state.plock_clipboard {
                if entry.instrument == instrument {
                    if plock_menu_action_row(ui, "Paste Plock", ACCENT).clicked() {
                        plock.masks.set_active(instrument, step, true);
                        plock
                            .field_masks
                            .set_raw(instrument, step, entry.field_mask);
                        for (field, &value) in entry.values.iter().enumerate() {
                            plock.values.set(instrument, step, field, value);
                        }
                    }
                }
            }
            return;
        }

        // ------ Mode indicator ------
        let mask = plock.field_masks.get(instrument, step);
        let all_bits = if FIELD_COUNT >= 64 {
            0xFFFFFFFFFFFFFFFFu64
        } else {
            (1u64 << FIELD_COUNT) - 1
        };
        let mode_text = if mask == 0 {
            "Linked to Global"
        } else if mask == all_bits {
            "Full Snapshot"
        } else {
            "Mixed"
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new("Mode").font(f_sans_med(10.0)).color(INK3));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(mode_text)
                        .font(f_mono_med(10.0))
                        .color(ACCENT),
                );
            });
        });
        ui.add_space(8.0);

        // ------ Volume (most used, shown first) ------
        {
            let vol_field = crate::instrument_registry::StandardField::Volume.plock_field_index();
            let mut vol_value = if plock.field_masks.is_set(instrument, step, vol_field) {
                plock.values.get(instrument, step, vol_field)
            } else {
                global.2
            };
            let overridden = plock.field_masks.is_set(instrument, step, vol_field);
            let vol_response = plock_menu_row(
                ui,
                "Volume",
                ACCENT,
                overridden,
                Some(&format!("{:.2}", vol_value)),
                |ui| {
                    ui.add(
                        LocalParamSlider::new(&mut vol_value, 0.0..=2.0)
                            .with_width(120.0)
                            .without_value(),
                    )
                },
            );
            if vol_response.changed() {
                plock.masks.set_active(instrument, step, true);
                plock.field_masks.set(instrument, step, vol_field);
                plock.set_field(instrument, step, vol_field, vol_value);
            }
        }

        // ------ Standard fields ------
        let get_global_value = |field: crate::instrument_registry::StandardField| -> f32 {
            match field {
                crate::instrument_registry::StandardField::Freq => global.0,
                crate::instrument_registry::StandardField::Decay => global.1,
                crate::instrument_registry::StandardField::Volume => global.2,
                crate::instrument_registry::StandardField::FilterFreq => global.3,
                crate::instrument_registry::StandardField::Attack => global.4,
                crate::instrument_registry::StandardField::Release => global.5,
                crate::instrument_registry::StandardField::DecayCurve => global.6,
                crate::instrument_registry::StandardField::ReleaseCurve => global.7,
                crate::instrument_registry::StandardField::Hold => global.8,
                crate::instrument_registry::StandardField::FilterEnvAmount => global.9,
                crate::instrument_registry::StandardField::FilterEnvDecay => global.10,
                crate::instrument_registry::StandardField::Analog => global.11,
                crate::instrument_registry::StandardField::Stereo => global.12,
            }
        };

        let is_bass_drum_plock = matches!(voice_idx, 0 | 11);
        let freq_in_notes_plock =
            is_bass_drum_plock && sound_settings.instruments[instrument].freq_mode();

        // Note display toggle for bass drums in plock (per-slot freq mode)
        if is_bass_drum_plock {
            let freq_in_notes = freq_in_notes_plock;
            let label = if freq_in_notes { "Notes" } else { "Hz" };
            plock_menu_row(ui, "Display", ACCENT, false, None, |ui| {
                if plock_menu_action_row(ui, label, ACCENT).clicked() {
                    sound_settings.instruments[instrument].set_freq_mode(!freq_in_notes);
                    sound_settings.bump_version();
                }
                ui.allocate_response(Vec2::new(1.0, 1.0), egui::Sense::hover())
            });
        }

        for def in inst_def.standard_params {
            if def.field == crate::instrument_registry::StandardField::Volume {
                continue;
            }
            let field_index = def.field.plock_field_index();
            let mut value = if plock.field_masks.is_set(instrument, step, field_index) {
                plock.values.get(instrument, step, field_index)
            } else {
                get_global_value(def.field)
            };
            let overridden = plock.field_masks.is_set(instrument, step, field_index);

            // Special case: frequency in note mode for bass drums
            if def.field == crate::instrument_registry::StandardField::Freq && freq_in_notes_plock {
                let ratio = inst_def.freq_display_ratio;
                let note_val = freq_to_note(value * ratio).round();
                let label = format!("{} {}", def.label, note_name(note_val));
                plock_menu_row(ui, &label, ACCENT, overridden, None, |ui| {
                    let mut changed = false;
                    if ui.small_button("-").clicked() {
                        let new_note = (note_val - 1.0).max(0.0);
                        value = note_to_freq(new_note) / ratio;
                        changed = true;
                    }
                    if ui.small_button("+").clicked() {
                        let new_note = (note_val + 1.0).min(127.0);
                        value = note_to_freq(new_note) / ratio;
                        changed = true;
                    }
                    if changed {
                        plock.set_field(instrument, step, field_index, value);
                    }
                    ui.allocate_response(Vec2::new(1.0, 1.0), egui::Sense::hover())
                });
                continue;
            }

            match &def.widget {
                crate::instrument_registry::ParamWidget::Slider {
                    min,
                    max,
                    logarithmic,
                    ..
                } => {
                    let value_text = format_value_for_plock(def.field, value, *min, *max);
                    let row_response = plock_menu_row(
                        ui,
                        def.label,
                        ACCENT,
                        overridden,
                        Some(&value_text),
                        |ui| {
                            ui.add(
                                LocalParamSlider::new(&mut value, *min..=*max)
                                    .logarithmic(*logarithmic)
                                    .with_width(120.0)
                                    .without_value(),
                            )
                        },
                    );
                    if row_response.changed() {
                        plock.masks.set_active(instrument, step, true);
                        plock.field_masks.set(instrument, step, field_index);
                        plock.set_field(instrument, step, field_index, value);
                    }
                }
                crate::instrument_registry::ParamWidget::Checkbox => {
                    plock_menu_row(ui, def.label, ACCENT, overridden, None, |ui| {
                        let mut checked = value >= 0.5;
                        let response = ui.add(egui::Checkbox::new(
                            &mut checked,
                            RichText::new("").font(f_sans_med(10.0)),
                        ));
                        if response.changed() {
                            value = if checked { 1.0 } else { 0.0 };
                        }
                        response
                    });
                    if overridden {
                        plock.set_field(instrument, step, field_index, value);
                    }
                }
            }
        }

        // ------ Algo ------
        {
            let algo_count = crate::instrument_registry::algo_count(voice_idx);
            if algo_count > 1 {
                let voice = DrumVoice::from_index(voice_idx).expect("valid voice index");
                let algos = synthesis::algos_for(voice);
                let algo_names: Vec<&str> = algos.iter().map(|a| a.name).collect();

                let mut algo_val = if plock.field_masks.is_set(instrument, step, 13) {
                    plock.values.get(instrument, step, 13) as u8
                } else {
                    params.algos()[instrument].value() as u8
                };
                let algo_overridden = plock.field_masks.is_set(instrument, step, 13);

                let max_algo = (algo_count - 1) as u8;
                if algo_val > max_algo {
                    algo_val = max_algo;
                }

                let mut selected = algo_val as usize;
                let current_name = algo_names.get(selected).copied().unwrap_or("?");
                let _algo_response = plock_menu_row(
                    ui,
                    "Algo",
                    ACCENT,
                    algo_overridden,
                    Some(current_name),
                    |ui| {
                        let (response, picked) =
                            styled_select(ui, "plock_algo", selected, &algo_names, 120.0);
                        if let Some(picked) = picked {
                            selected = picked;
                        }
                        response
                    },
                );
                if selected != algo_val as usize {
                    plock.masks.set_active(instrument, step, true);
                    plock.field_masks.set(instrument, step, 13);
                    plock.set_field(instrument, step, 13, selected as f32);
                }
            }
        }

        // ------ Special params ------
        let special_defs = crate::instrument_registry::special_params(voice_idx);
        for def in special_defs {
            if def.special_index >= 8 {
                continue;
            }
            let field = SPECIAL_FIELD_START + def.special_index;
            let value = if plock.field_masks.is_set(instrument, step, field) {
                plock.values.get(instrument, step, field)
            } else {
                inst.special_value(def.special_index)
            };
            let overridden = plock.field_masks.is_set(instrument, step, field);
            let label_lower = def.label.to_lowercase();

            let new_value = if label_lower.contains("saturation type") {
                let (_, picked) = plock_menu_enum_row(
                    ui,
                    def.label,
                    ACCENT,
                    overridden,
                    value,
                    &[
                        "None",
                        "SoftClip",
                        "Valve",
                        "Transistor",
                        "HardClip",
                        "Tape",
                    ],
                    def.name,
                );
                picked
            } else if label_lower.contains("noise type") {
                let (_, picked) = plock_menu_enum_row(
                    ui,
                    def.label,
                    ACCENT,
                    overridden,
                    value,
                    &["White", "Pink", "Brown", "Blue"],
                    def.name,
                );
                picked
            } else if label_lower.contains("click type") {
                let (_, picked) = plock_menu_enum_row(
                    ui,
                    def.label,
                    ACCENT,
                    overridden,
                    value,
                    &["Soft", "Medium", "Hard"],
                    def.name,
                );
                picked
            } else {
                let mut value = value;
                let log = def.min > 0.0 && def.max / def.min >= 20.0;
                let value_text = format_value_for_plock_special(value, def.min, def.max);
                let response =
                    plock_menu_row(ui, def.label, ACCENT, overridden, Some(&value_text), |ui| {
                        ui.add(
                            LocalParamSlider::new(&mut value, def.min..=def.max)
                                .logarithmic(log)
                                .with_width(120.0)
                                .without_value(),
                        )
                    });
                if response.changed() {
                    Some(value)
                } else {
                    None
                }
            };

            if let Some(new_value) = new_value {
                let clamped = new_value.clamp(def.min, def.max);
                plock.masks.set_active(instrument, step, true);
                plock.field_masks.set(instrument, step, field);
                plock.set_field(instrument, step, field, clamped);
            }
        }

        // ------ Actions ------
        ui.add_space(8.0);
        if plock_menu_action_row(ui, "Copy Plock", ACCENT).clicked() {
            let field_mask = plock.field_masks.get_raw(instrument, step);
            let mut values = Vec::with_capacity(crate::plock::FIELD_COUNT);
            for field in 0..crate::plock::FIELD_COUNT {
                values.push(plock.values.get(instrument, step, field));
            }
            state.plock_clipboard = Some(SinglePlockClipboard {
                instrument,
                field_mask,
                values,
            });
        }
        if let Some(ref entry) = state.plock_clipboard {
            if entry.instrument == instrument {
                if plock_menu_action_row(ui, "Paste Plock", ACCENT).clicked() {
                    plock.masks.set_active(instrument, step, true);
                    plock
                        .field_masks
                        .set_raw(instrument, step, entry.field_mask);
                    for (field, &value) in entry.values.iter().enumerate() {
                        plock.values.set(instrument, step, field, value);
                    }
                }
            }
        }
        if plock_menu_action_row(ui, "Clear Plock", Color32::from_rgb(255, 80, 80)).clicked() {
            plock.clear(instrument, step);
        }
    });
}

fn draw_fusion_morph_menu(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    fusion_index: usize,
    step: usize,
    state: &mut EditorUIState,
) {
    use crate::plock::SPECIAL_FIELD_START;

    const ACCENT: Color32 = PL_LINK;
    // `instrument` is a SLOT index; schema lookups use the slot's voice index.
    let voice_idx = schema_voice_idx(params, instrument);
    let inst_def = &crate::instrument_registry::INSTRUMENTS[voice_idx];
    let title = format!("Morph {}", inst_def.label);

    let mut new_fusions = pattern.load_fusions(instrument);
    if new_fusions.get(fusion_index).is_none() {
        return;
    }

    plock_menu_frame(ui, ACCENT, |ui| {
        if plock_menu_header(ui, &title, step, ACCENT) {
            state.plock_popup = None;
        }

        // Mode indicator
        let mode_text = if new_fusions
            .get(fusion_index)
            .map(|g| g.morph_count)
            .unwrap_or(0)
            == 0
        {
            "Off"
        } else {
            "On"
        };
        ui.horizontal(|ui| {
            ui.label(RichText::new("Mode").font(f_sans_med(10.0)).color(INK3));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(mode_text)
                        .font(f_mono_med(10.0))
                        .color(ACCENT),
                );
            });
        });
        ui.add_space(8.0);

        // Disable morphing
        if plock_menu_action_row(ui, "Disable Morphing", Color32::from_rgb(255, 80, 80)).clicked() {
            if let Some(g) = new_fusions.get_mut(fusion_index) {
                g.morph_count = 0;
                g.morph_targets = [MorphTarget::default(); 4];
                pattern.store_fusions(instrument, &new_fusions);
                state.plock_popup = None;
            }
        }
        ui.add_space(8.0);

        // Volume
        {
            let vol_field = crate::instrument_registry::StandardField::Volume.plock_field_index();
            let (mut vol_value, is_target) = fusion_morph_state(
                &new_fusions,
                fusion_index,
                vol_field,
                params,
                sound_settings,
                instrument,
            );
            vol_value = vol_value.clamp(0.0, 2.0);
            let vol_response = plock_menu_row(
                ui,
                "Volume",
                ACCENT,
                is_target,
                Some(&format!("{:.2}", vol_value)),
                |ui| {
                    let slider = ui.add(
                        LocalParamSlider::new(&mut vol_value, 0.0..=2.0)
                            .with_width(96.0)
                            .without_value(),
                    );
                    if is_target {
                        if ui.small_button("×").clicked() {
                            if let Some(g) = new_fusions.get_mut(fusion_index) {
                                g.remove_morph_target(vol_field);
                                pattern.store_fusions(instrument, &new_fusions);
                            }
                        }
                    }
                    slider
                },
            );
            if vol_response.changed() {
                if let Some(g) = new_fusions.get_mut(fusion_index) {
                    g.set_morph_target(vol_field, vol_value);
                    pattern.store_fusions(instrument, &new_fusions);
                }
            }
        }

        // Standard fields (per-slot freq display mode)
        let is_bass_drum = matches!(voice_idx, 0 | 11);
        let freq_in_notes = is_bass_drum && sound_settings.instruments[instrument].freq_mode();

        // Note display toggle for bass drums
        if is_bass_drum {
            let label = if freq_in_notes { "Notes" } else { "Hz" };
            plock_menu_row(ui, "Display", ACCENT, false, None, |ui| {
                let response = plock_menu_action_row(ui, label, ACCENT);
                if response.clicked() {
                    sound_settings.instruments[instrument].set_freq_mode(!freq_in_notes);
                    sound_settings.bump_version();
                }
                response
            });
        }

        for def in inst_def.standard_params {
            if def.field == crate::instrument_registry::StandardField::Volume {
                continue;
            }
            let field_index = def.field.plock_field_index();
            let (mut value, is_target) = fusion_morph_state(
                &new_fusions,
                fusion_index,
                field_index,
                params,
                sound_settings,
                instrument,
            );

            // Special case: frequency in note mode for bass drums
            if def.field == crate::instrument_registry::StandardField::Freq && freq_in_notes {
                let ratio = inst_def.freq_display_ratio;
                let note_val = freq_to_note(value * ratio).round();
                let label = format!("{} {}", def.label, note_name(note_val));
                plock_menu_row(ui, &label, ACCENT, is_target, None, |ui| {
                    let mut changed = false;
                    if ui.small_button("-").clicked() {
                        let new_note = (note_val - 1.0).max(0.0);
                        value = note_to_freq(new_note) / ratio;
                        changed = true;
                    }
                    if ui.small_button("+").clicked() {
                        let new_note = (note_val + 1.0).min(127.0);
                        value = note_to_freq(new_note) / ratio;
                        changed = true;
                    }
                    if is_target && ui.small_button("×").clicked() {
                        if let Some(g) = new_fusions.get_mut(fusion_index) {
                            g.remove_morph_target(field_index);
                            pattern.store_fusions(instrument, &new_fusions);
                        }
                    }
                    if changed {
                        if let Some(g) = new_fusions.get_mut(fusion_index) {
                            g.set_morph_target(field_index, value);
                            pattern.store_fusions(instrument, &new_fusions);
                        }
                    }
                    ui.allocate_response(Vec2::new(1.0, 1.0), egui::Sense::hover())
                });
                continue;
            }

            match &def.widget {
                crate::instrument_registry::ParamWidget::Slider {
                    min,
                    max,
                    logarithmic,
                    ..
                } => {
                    let value_text = format_value_for_plock(def.field, value, *min, *max);
                    value = value.clamp(*min, *max);
                    let row_response =
                        plock_menu_row(ui, def.label, ACCENT, is_target, Some(&value_text), |ui| {
                            let slider = ui.add(
                                LocalParamSlider::new(&mut value, *min..=*max)
                                    .logarithmic(*logarithmic)
                                    .with_width(96.0)
                                    .without_value(),
                            );
                            if is_target {
                                if ui.small_button("×").clicked() {
                                    if let Some(g) = new_fusions.get_mut(fusion_index) {
                                        g.remove_morph_target(field_index);
                                        pattern.store_fusions(instrument, &new_fusions);
                                    }
                                }
                            }
                            slider
                        });
                    if row_response.changed() {
                        if let Some(g) = new_fusions.get_mut(fusion_index) {
                            g.set_morph_target(field_index, value);
                            pattern.store_fusions(instrument, &new_fusions);
                        }
                    }
                }
                crate::instrument_registry::ParamWidget::Checkbox => {
                    plock_menu_row(ui, def.label, ACCENT, is_target, None, |ui| {
                        let mut checked = value >= 0.5;
                        let response = ui.add(egui::Checkbox::new(
                            &mut checked,
                            RichText::new("").font(f_sans_med(10.0)),
                        ));
                        if response.changed() {
                            value = if checked { 1.0 } else { 0.0 };
                            if let Some(g) = new_fusions.get_mut(fusion_index) {
                                g.set_morph_target(field_index, value);
                                pattern.store_fusions(instrument, &new_fusions);
                            }
                        }
                        response
                    });
                }
            }
        }

        // Special params (continuous only — discrete params can't be morphed).
        // Also skip any special param whose plock field overlaps a standard field
        // (e.g. special_index 4 → field 18 which is also Attack).
        let standard_field_indices: std::collections::HashSet<usize> = inst_def
            .standard_params
            .iter()
            .map(|def| def.field.plock_field_index())
            .collect();
        let special_defs = crate::instrument_registry::special_params(voice_idx);
        for def in special_defs {
            if def.special_index >= 8 || !def.continuous {
                continue;
            }
            let field = SPECIAL_FIELD_START + def.special_index;
            if standard_field_indices.contains(&field) {
                continue;
            }
            let (mut value, is_target) = fusion_morph_state(
                &new_fusions,
                fusion_index,
                field,
                params,
                sound_settings,
                instrument,
            );
            value = value.clamp(def.min, def.max);
            let log = def.min > 0.0 && def.max / def.min >= 20.0;
            let value_text = format_value_for_plock_special(value, def.min, def.max);
            let special_response =
                plock_menu_row(ui, def.label, ACCENT, is_target, Some(&value_text), |ui| {
                    let slider = ui.add(
                        LocalParamSlider::new(&mut value, def.min..=def.max)
                            .logarithmic(log)
                            .with_width(96.0)
                            .without_value(),
                    );
                    if is_target {
                        if ui.small_button("×").clicked() {
                            if let Some(g) = new_fusions.get_mut(fusion_index) {
                                g.remove_morph_target(field);
                                pattern.store_fusions(instrument, &new_fusions);
                            }
                        }
                    }
                    slider
                });
            if special_response.changed() {
                if let Some(g) = new_fusions.get_mut(fusion_index) {
                    g.set_morph_target(field, value);
                    pattern.store_fusions(instrument, &new_fusions);
                }
            }
        }
    });
}

fn draw_plock_popup(
    ctx: &egui::Context,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
) {
    let mut popup = match state.plock_popup {
        Some(p) => p,
        None => return,
    };

    let area_id = egui::Id::new("plock_popup");
    let response = egui::Area::new(area_id)
        .kind(egui::UiKind::Menu)
        .order(egui::Order::Foreground)
        .fixed_pos(popup.screen_pos)
        .sense(egui::Sense::hover())
        .show(ctx, |ui| {
            // Outer border: draw a slightly larger rounded rect behind the panel.
            let content_response = egui::Frame::NONE
                .fill(P_ACTIVE)
                .corner_radius(RADIUS_PANEL)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_min_width(260.0);
                    ui.set_max_width(284.0);

                    let inst = popup.instrument;
                    let step = popup.step;
                    let fusions = pattern.load_fusions(inst);
                    let fusion_group = fusions.iter().find(|g| {
                        let start = g.start_cell;
                        let end = g.end_cell;
                        (start as usize) <= step && step <= (end as usize)
                    });
                    let fusion_info = fusions.iter().enumerate().find(|(_, g)| {
                        (g.start_cell as usize) <= step && step <= (g.end_cell as usize)
                    });

                    if state.sequencer_mode {
                        draw_sequencer_plock_menu(
                            ui,
                            params,
                            setter,
                            inst,
                            step,
                            state,
                            fusion_group.is_some(),
                        );
                    } else {
                        if let Some((idx, group)) = fusion_info {
                            if popup.morph_menu {
                                draw_fusion_morph_menu(
                                    ui,
                                    pattern,
                                    sound_settings,
                                    params,
                                    setter,
                                    inst,
                                    idx,
                                    step,
                                    state,
                                );
                            } else {
                                plock_menu_frame(ui, PL_LINK, |ui| {
                                    if plock_menu_header(
                                        ui,
                                        &format!(
                                            "Fusion {}-{}",
                                            group.start_cell + 1,
                                            group.end_cell + 1
                                        ),
                                        step,
                                        PL_LINK,
                                    ) {
                                        state.plock_popup = None;
                                    }

                                    let morph_active = group.morph_count > 0;
                                    let morph_label = if morph_active {
                                        let morphable =
                                            crate::instrument_registry::morphable_fields(
                                                schema_voice_idx(params, inst),
                                            );
                                        let names: Vec<&str> = group.morph_targets
                                            [..group.morph_count as usize]
                                            .iter()
                                            .map(|t| {
                                                morphable
                                                    .iter()
                                                    .find(|f| f.field_index == t.field as usize)
                                                    .map(|f| f.label)
                                                    .unwrap_or("?")
                                            })
                                            .collect();
                                        format!("Morphing ({})", names.join(", "))
                                    } else {
                                        "Morphing".to_string()
                                    };
                                    if plock_menu_action_row(ui, &morph_label, PL_LINK).clicked() {
                                        popup.morph_menu = true;
                                        state.plock_popup = Some(popup);
                                    }
                                    if plock_menu_action_row(ui, "Edit Fusion Steps", PL_LINK)
                                        .clicked()
                                    {
                                        state.fusion_editing = Some((inst, idx));
                                        state.plock_popup = None;
                                    }
                                    if plock_menu_action_row(
                                        ui,
                                        "Delete Fusion",
                                        Color32::from_rgb(255, 80, 80),
                                    )
                                    .clicked()
                                    {
                                        let mut new_fusions = fusions.clone();
                                        if idx < new_fusions.len() {
                                            new_fusions.remove(idx);
                                            pattern.store_fusions(inst, &new_fusions);
                                        }
                                        state.plock_popup = None;
                                    }
                                });
                                ui.separator();

                                // Also show the source-step plock menu below.
                                draw_plock_menu(
                                    ui,
                                    plock,
                                    sound_settings,
                                    params,
                                    setter,
                                    inst,
                                    step,
                                    state,
                                );
                            }
                        } else {
                            draw_plock_menu(
                                ui,
                                plock,
                                sound_settings,
                                params,
                                setter,
                                inst,
                                step,
                                state,
                            );
                        }
                    }
                })
                .response;

            let border_rect = content_response.rect.expand2(egui::Vec2::new(1.0, 1.0));
            ui.painter().rect_stroke(
                border_rect,
                RADIUS_PANEL + 1.0,
                egui::Stroke::new(1.0, LINE2),
                egui::StrokeKind::Inside,
            );
            content_response
        })
        .response;

    // Close popup on click outside.
    if response.clicked_elsewhere() {
        state.plock_popup = None;
    }
}

fn draw_sequencer_plock_menu(
    ui: &mut egui::Ui,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    step: usize,
    state: &mut EditorUIState,
    stutter_disabled: bool,
) {
    use crate::plock::{SequencerStepParams, StepCondition};

    const ACCENT: Color32 = SEQPL;
    // `instrument` is a SLOT index; the label comes from the slot's voice schema.
    let inst_def = &crate::instrument_registry::INSTRUMENTS[schema_voice_idx(params, instrument)];
    let title = format!("Seq Plock {}", inst_def.label);

    plock_menu_frame(ui, ACCENT, |ui| {
        if plock_menu_header(ui, &title, step, ACCENT) {
            state.plock_popup = None;
        }

        let seq_plock = &params.seq_plock_state.state;
        let has_seq_plock = seq_plock.is_active(instrument, step);
        let current = seq_plock.get(instrument, step).unwrap_or_default();
        let mut changed_this_frame = false;

        // Mode indicator
        ui.horizontal(|ui| {
            ui.label(RichText::new("Mode").font(f_sans_med(10.0)).color(INK3));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mode_text = if has_seq_plock { "Active" } else { "Inactive" };
                ui.label(
                    RichText::new(mode_text)
                        .font(f_mono_med(10.0))
                        .color(ACCENT),
                );
            });
        });
        ui.add_space(8.0);

        // Probability
        {
            let mut prob = current.probability;
            let prob_text = format!("{:.0}%", prob * 100.0);
            let prob_response = plock_menu_row(
                ui,
                "Probability",
                ACCENT,
                has_seq_plock,
                Some(&prob_text),
                |ui| {
                    ui.add(
                        LocalParamSlider::new(&mut prob, 0.0..=1.0)
                            .with_width(86.0)
                            .without_value(),
                    )
                },
            );
            if prob_response.changed() {
                seq_plock.set_probability(instrument, step, prob);
                changed_this_frame = true;
            }
        }

        // Stutter
        if stutter_disabled {
            if has_seq_plock && current.stutter_count != 1 {
                let mut fixed = current;
                fixed.stutter_count = 1;
                seq_plock.set(instrument, step, &fixed);
            }
            plock_menu_row(ui, "Stutter", ACCENT, false, None, |ui| {
                ui.label(
                    RichText::new("disabled on fusion")
                        .font(f_sans_med(10.0))
                        .color(INK3),
                );
                ui.allocate_response(Vec2::new(1.0, 1.0), egui::Sense::hover())
            });
        } else {
            let mut stutter = current.stutter_count.max(1) as f32;
            let stutter_text = format!("{}x", stutter as i32);
            let stutter_response = plock_menu_row(
                ui,
                "Stutter",
                ACCENT,
                has_seq_plock && current.stutter_count != 1,
                Some(&stutter_text),
                |ui| {
                    ui.add(
                        LocalParamSlider::new(&mut stutter, 1.0..=16.0)
                            .with_width(86.0)
                            .without_value(),
                    )
                },
            );
            if stutter_response.changed() {
                let new_stutter = stutter.round() as u8;
                seq_plock.set_stutter(instrument, step, new_stutter);
                changed_this_frame = true;
            }
        }

        // Condition
        ui.add_space(8.0);
        ui.label(RichText::new("Condition").font(f_sans_sb(10.0)).color(INK2));
        ui.add_space(6.0);

        let all_conditions = StepCondition::all();
        let grid_id = format!("condition_grid_{}_{}", instrument, step);
        let available_w = ui.available_width();
        let button_w = (available_w - 16.0) / 3.0;
        egui::Grid::new(grid_id)
            .num_columns(3)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                for (idx, cond) in all_conditions.iter().copied().enumerate() {
                    let selected = current.condition == cond;
                    let text_color = if selected { ACCENT } else { INK2 };
                    let fill = if selected { PANEL2 } else { PANEL2 };
                    let stroke_color = if selected { ACCENT } else { LINE2 };
                    if ui
                        .add_sized(
                            Vec2::new(button_w.max(1.0), 26.0),
                            egui::Button::new(
                                RichText::new(cond.label())
                                    .font(f_sans_med(9.5))
                                    .color(text_color),
                            )
                            .fill(fill)
                            .stroke(egui::Stroke::new(1.0, stroke_color))
                            .corner_radius(6.0),
                        )
                        .clicked()
                    {
                        seq_plock.set_condition(instrument, step, cond);
                        changed_this_frame = true;
                    }
                    if (idx + 1) % 3 == 0 {
                        ui.end_row();
                    }
                }
            });

        // Actions
        ui.add_space(8.0);
        if has_seq_plock || changed_this_frame {
            if plock_menu_action_row(ui, "Clear Seq Plock", Color32::from_rgb(255, 80, 80))
                .clicked()
            {
                seq_plock.clear(instrument, step);
            }
        } else {
            if plock_menu_action_row(ui, "Create Seq Plock", ACCENT).clicked() {
                seq_plock.set(instrument, step, &SequencerStepParams::default());
            }
        }
    });
}
