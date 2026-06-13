use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32, RichText, Vec2},
    resizable_window::ResizableWindow,
    widgets::ParamSlider,
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
    pattern_bank::SLOT_COUNT,
    plock::PlockState,
    preset_dumps,
    sequencer::{FusedGroup, Pattern, SharedPattern},
    sound_settings::SoundSettingsState,
    synthesis::{self, DrumVoice, VoiceSettings},
    DrumFlashParams, BUILD_ID,
};

mod design_system;
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

fn note_name(note: f32) -> String {
    let note = note.round() as i32;
    let octave = (note / 12) - 1;
    let note_idx = note % 12;
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", names[note_idx as usize], octave)
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
        ("sans_400", include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf")),
        ("sans_500", include_bytes!("../assets/fonts/IBMPlexSans-Medium.ttf")),
        ("sans_600", include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf")),
        ("sans_700", include_bytes!("../assets/fonts/IBMPlexSans-Bold.ttf")),
        ("mono_400", include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf")),
        ("mono_500", include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf")),
        ("mono_600", include_bytes!("../assets/fonts/IBMPlexMono-SemiBold.ttf")),
    ];
    for (name, bytes) in faces {
        fonts
            .font_data
            .insert(name.to_string(), Arc::new(egui::FontData::from_static(bytes)));
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
) {
    let locked = params.lane_length_locks.is_locked(instrument);
    let raw = length_param.value() as usize;
    let mut length_value = if locked && master_length > raw {
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

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct EditorUIState {
    selected_instrument: usize,
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
    /// Start cell of a fusion selection (Shift+click), per instrument.
    fusion_selection_start: [Option<usize>; DrumVoice::COUNT],
    /// Currently editing fusion group: (instrument, group_index).
    fusion_editing: Option<(usize, usize)>,
}

pub fn create_editor(
    params: Arc<DrumFlashParams>,
    current_step: Arc<AtomicU32>,
    current_steps: Arc<[AtomicU32; DrumVoice::COUNT]>,
    pattern: Arc<SharedPattern>,
    voice_test_triggers: Arc<[AtomicBool; DrumVoice::COUNT]>,
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
            visuals.window_stroke = egui::Stroke::new(1.0, LINE);
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
                .min_size(Vec2::new(1480.0, 800.0))
                .fixed_size(Vec2::new(1480.0, 800.0))
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

                    ui.allocate_ui_at_rect(left_rect.shrink2(Vec2::new(14.0, 14.0)), |ui| {
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
                    });

                    ui.allocate_ui_at_rect(right_rect, |ui| {
                        ui.painter().rect_filled(ui.max_rect(), 0.0, PANEL);
                        ui.set_width(right_w);
                        ui.set_height(body_h);
                        draw_sound_panel(ui, &sound_settings_for_ui, &params_for_ui, setter, state);
                    });
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
    ui.allocate_ui_at_rect(content_rect, |ui| {
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
            let sel = led_segmented(ui, &["Internal", "Ext MIDI"], if internal { 0 } else { 1 });
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
    });
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

        ui.add_space(16.0);

        // MIDI export chips (right side)
        if chip_button(ui, "Export MIDI", false, BLUE).clicked() {
            let bpm = params.bpm.value();
            let pattern_length = params.pattern_length.value() as usize;
            match export_midi_to_documents(pattern, bpm, pattern_length) {
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

        let drag_response = chip_button(ui, "Drag MIDI", false, BLUE)
            .on_hover_text("Drag the current pattern into your DAW");
        if drag_response.clicked() || drag_response.drag_started() {
            let bpm = params.bpm.value();
            let pattern_length = params.pattern_length.value() as usize;
            match export_midi_to_documents(pattern, bpm, pattern_length)
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
    song_mode: &Arc<AtomicBool>,
    song_position: &Arc<AtomicU32>,
) {
    let panel_w = ui.available_width();
    let panel_h = 132.0;
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
    ui.allocate_ui_at_rect(header_rect.shrink2(Vec2::new(12.0, 0.0)), |ui| {
        ui.set_clip_rect(header_rect);
        ui.horizontal(|ui| {
            ui.set_height(42.0);
            ui.spacing_mut().item_spacing.x = 0.0;
            let is_song = params.song_mode.value();
            // Generator | Song segmented tabs
            let _options = ["Generator", "Song"];
            let selected = if is_song { 1 } else { 0 };
            let new_selected = p_lock_mode_segmented(ui, selected);
            if new_selected != selected {
                setter.set_parameter(&params.song_mode, new_selected == 1);
            }

            ui.add_space(12.0);

            // Meta text
            let meta = if params.song_mode.value() {
                if let Ok(bank) = params.pattern_bank.bank.lock() {
                    let total_reps = bank.song.steps[..bank.song.length as usize]
                        .iter()
                        .filter(|&&s| s >= 0)
                        .count();
                    let blocks = bank.song.length as usize;
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

            ui.add_space(ui.available_width() - 104.0);

            // Song Enabled toggle (only visible in Song mode)
            if params.song_mode.value() {
                let song_enabled = song_mode.load(Ordering::Relaxed);
                let tog = led_toggle(ui, "Song Enabled", song_enabled);
                if tog.clicked() {
                    song_mode.store(!song_enabled, Ordering::Relaxed);
                }
            }
        });
    });

    let body_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 12.0, rect.top() + 51.0),
        egui::pos2(rect.right() - 12.0, rect.bottom() - 10.0),
    );
    ui.allocate_ui_at_rect(body_rect, |ui| {
        ui.set_clip_rect(body_rect);
        ui.set_width(body_rect.width());
        ui.set_height(body_rect.height());
        if params.song_mode.value() {
            draw_song_editor(ui, setter, params, state, song_mode, song_position);
        } else {
            draw_generator_panel_content(ui, setter, params, pattern, state);
        }
    });
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
        ui.spacing_mut().item_spacing.y = 9.0;
        draw_preset_bar(ui, pattern, params, setter, state);
        draw_generator_bar(ui, setter, params, pattern, state);
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
        if chip_button(ui, "⟳ Random", true, PL_LINK).clicked() {
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
    const SLIDER_W: f32 = 46.0;
    const LABEL_W: f32 = 30.0;
    const GEN_TYPE_W: f32 = 104.0;
    const STYLE_W: f32 = 88.0;
    const GEN_BTN_W: f32 = 110.0;
    const INNER_GAP: f32 = 5.0;

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;

        // --- Row 1: generator / type / A / B / sliders ---
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = INNER_GAP;
            ui.set_height(CTL_HEIGHT);

            genrow_label(ui, "Generator", 62.0);
            enum_combo_compact(ui, setter, &params.generator_type, "gen_type", GEN_TYPE_W);
            ui.add_space(2.0);
            genrow_label(ui, "Type", 30.0);
            enum_combo_compact(ui, setter, &params.style_primary, "style_a", STYLE_W);
            ui.add_space(2.0);
            genrow_label(ui, "A", 14.0);
            enum_combo_compact(ui, setter, &params.style_secondary, "style_b", STYLE_W);
            ui.add_space(2.0);
            genrow_label(ui, "B", 14.0);

            ui.add_space(18.0);

            // Morph slider (A -> B) has no label
            ui.add(
                ParamSlider::for_param(&params.style_mix, setter)
                    .with_width(SLIDER_W)
                    .without_value(),
            );
            ui.add_space(INNER_GAP);

            genrow_label(ui, "Mix", LABEL_W);
            ui.add(
                ParamSlider::for_param(&params.gen_density, setter)
                    .with_width(SLIDER_W)
                    .without_value(),
            );
            ui.add_space(INNER_GAP);

            genrow_label(ui, "Dens", LABEL_W);
            ui.add(
                ParamSlider::for_param(&params.gen_variation, setter)
                    .with_width(SLIDER_W)
                    .without_value(),
            );
            ui.add_space(INNER_GAP);

            genrow_label(ui, "Var", LABEL_W);
        });

        // --- Row 2: GENERATE button aligned right ---
        ui.horizontal(|ui| {
            ui.set_height(CTL_HEIGHT);
            ui.add_space(ui.available_width() - GEN_BTN_W);
            let gen_btn = egui::Button::new(RichText::new("GENERATE").font(f_sans_sb(11.0)))
                .min_size(Vec2::new(GEN_BTN_W, CTL_HEIGHT))
                .fill(BLUE)
                .stroke(egui::Stroke::new(1.0, BLUE))
                .corner_radius(6.0);
            if ui.add(gen_btn).clicked() {
                params.plock_state.state.clear_all();
                params.seq_plock_state.state.clear_all();
                let gen_params = generator::GeneratorParams {
                    generator_type: params.generator_type.value(),
                    style_primary: params.style_primary.value(),
                    style_secondary: params.style_secondary.value(),
                    style_mix: params.style_mix.value(),
                    density: params.gen_density.value(),
                    variation: params.gen_variation.value(),
                };
                let seed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos() as u64)
                    .unwrap_or(0);
                let mut rng_state = seed;
                let mut rng = || {
                    rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (rng_state as f32) / (u64::MAX as f32)
                };
                let generated = generator::generate(&gen_params, &mut rng);
                let pattern_length = params.pattern_length.value() as usize;
                clear_all_fusions(pattern);
                load_pattern_for_ui_with_length(pattern, &generated, pattern_length);
                state.last_loaded_slot = None;
            }
        });
    });
}
fn draw_song_editor(
    ui: &mut egui::Ui,
    _setter: &ParamSetter,
    params: &DrumFlashParams,
    _state: &mut EditorUIState,
    song_mode: &Arc<AtomicBool>,
    song_position: &Arc<AtomicU32>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Song Sequence").strong().size(12.0));
        ui.add_space(8.0);

        // Loop toggle
        if let Ok(mut bank) = params.pattern_bank.bank.lock() {
            let loop_enabled = bank.song.loop_enabled;
            let btn = egui::Button::new(RichText::new("Loop").size(10.0))
                .min_size(Vec2::new(40.0, 22.0))
                .fill(if loop_enabled {
                    Color32::from_rgb(74, 158, 255)
                } else {
                    PANEL2
                })
                .stroke(egui::Stroke::new(1.0, LINE2))
                .corner_radius(5.0);
            if ui.add(btn).clicked() {
                bank.song.loop_enabled = !loop_enabled;
            }
        }

        ui.add_space(8.0);

        // Song length control
        if let Ok(mut bank) = params.pattern_bank.bank.lock() {
            let len = bank.song.length;
            ui.label(
                RichText::new(format!("Len: {}", len))
                    .size(10.0)
                    .monospace(),
            );
            if ui.button("+").clicked() && bank.song.length < 64 {
                bank.song.length += 1;
            }
            if ui.button("-").clicked() && bank.song.length > 0 {
                bank.song.length -= 1;
            }
        }
    });

    ui.add_space(4.0);

    // Song steps grid
    let current_song_pos = song_position.load(Ordering::Relaxed) as usize;

    if let Ok(mut bank) = params.pattern_bank.bank.lock() {
        let len = bank.song.length as usize;
        let steps_per_row = 16_usize;
        let rows = (len + steps_per_row - 1) / steps_per_row;

        for row in 0..rows.max(1) {
            ui.horizontal(|ui| {
                for col in 0..steps_per_row {
                    let step_idx = row * steps_per_row + col;
                    if step_idx >= len {
                        break;
                    }

                    let is_current =
                        step_idx == current_song_pos && song_mode.load(Ordering::Relaxed);
                    let slot = bank.song.steps[step_idx];
                    let occupied = slot >= 0
                        && (slot as usize) < SLOT_COUNT
                        && bank.slots[slot as usize].occupied;

                    let text = if slot < 0 {
                        "--".to_string()
                    } else {
                        format!("P{}", slot + 1)
                    };

                    let btn = egui::Button::new(RichText::new(text).size(9.0).monospace())
                        .min_size(Vec2::new(28.0, 22.0))
                        .fill(if is_current {
                            Color32::from_rgb(255, 100, 100)
                        } else if occupied {
                            PANEL2
                        } else {
                            Color32::from_rgb(28, 28, 36)
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if is_current {
                                Color32::from_rgb(255, 150, 150)
                            } else {
                                LINE2
                            },
                        ))
                        .corner_radius(4.0);

                    let response = ui.add(btn);
                    if response.clicked() {
                        // Cycle through P1-P8 and empty
                        let next_slot = if slot < 0 {
                            0
                        } else if (slot as usize) >= SLOT_COUNT - 1 {
                            -1
                        } else {
                            slot + 1
                        };
                        bank.song.set_step(step_idx, next_slot);
                    }
                    if response.secondary_clicked() {
                        // Right click to clear
                        bank.song.set_step(step_idx, -1);
                    }
                }
            });
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Pattern grid with per-row Hum/Push/Len
// ---------------------------------------------------------------------------------------------------------------
fn draw_grid_v2(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    voice_test_triggers: &[AtomicBool; DrumVoice::COUNT],
    current_step: &AtomicU32,
    _current_steps: &[AtomicU32; DrumVoice::COUNT],
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
            let hums: [&FloatParam; DrumVoice::COUNT] =
                std::array::from_fn(|i| params.humanizes()[i]);
            let pushes: [&FloatParam; DrumVoice::COUNT] =
                std::array::from_fn(|i| params.pushes()[i]);
            let lengths: [&IntParam; DrumVoice::COUNT] =
                std::array::from_fn(|i| params.lengths()[i]);

            for inst in 0..DrumVoice::COUNT {
                let row = &mixer_rows[inst];
                let fusions = pattern.load_fusions(inst);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.set_height(LANE_H);

                    draw_seq_grip_v2(ui, grip_w, LANE_H);

                    let selected = state.selected_instrument == inst;
                    let name_response = draw_lane_name_v2(
                        ui,
                        name_w,
                        selected,
                        crate::instrument_registry::INSTRUMENTS[inst].label,
                    )
                    .on_hover_text(crate::instrument_registry::INSTRUMENTS[inst].full_name);
                    if name_response.clicked() {
                        state.selected_instrument = inst;
                    }

                    let inst_state = &sound_settings.instruments[inst];
                    let mut lane_vol = f32::from_bits(inst_state.volume.load(Ordering::Relaxed));
                    if draw_mini_value_slider(
                        ui,
                        &mut lane_vol,
                        0.0,
                        2.0,
                        vol_w,
                        BLUE,
                        "Lane Volume",
                    )
                    .changed()
                    {
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
                            state.selected_instrument = inst;
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
                            state.selected_instrument = inst;
                        }
                        if draw_tag_button_v2(ui, "T", BLUE, Color32::WHITE, false, "Test").clicked()
                        {
                            voice_test_triggers[inst].store(true, Ordering::Release);
                            if params.auto_edit.value() {
                                state.selected_instrument = inst;
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = GAP_TIGHT;
                        for local_step in 0..16 {
                            let global_step = page_offset + local_step;
                            let beyond_len = global_step >= master_length;
                            let fusion_info = fusion_containing(&fusions, global_step);
                            let fusion_group = fusion_info.map(|(_, group)| group);
                            let source_step = fusion_group
                                .map(|group| group.start_cell as usize)
                                .unwrap_or(global_step);
                            let is_fusion_start = fusion_group
                                .map(|group| group.is_start(global_step))
                                .unwrap_or(false);
                            let is_fusion_mid = fusion_group.is_some() && !is_fusion_start;

                            let active = !beyond_len && pattern.is_active(source_step, inst);
                            let is_current = fusion_group
                                .map(|group| group.contains(play_step))
                                .unwrap_or(play_step == global_step);
                            let has_sound_plock =
                                !beyond_len && plock.masks.is_active(inst, source_step);
                            let field_mask = if has_sound_plock {
                                plock.field_masks.get(inst, source_step)
                            } else {
                                0
                            };
                            let all_bits = (1u64 << crate::plock::FIELD_COUNT) - 1;
                            let is_snapshot = has_sound_plock && field_mask == all_bits;
                            let has_seq_plock = !beyond_len
                                && params.seq_plock_state.state.is_active(inst, source_step);
                            let selection_start = fusion_mode_active
                                && state.fusion_selection_start[inst] == Some(global_step);

                            let (fill, stroke) = step_colors_v2(
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
                            );
                            let text = if is_fusion_start {
                                fusion_group
                                    .map(|g| g.step_count.to_string())
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };
                            let response = draw_step_cell_v2(
                                ui,
                                Vec2::new(cell_w, STEP_H),
                                fill,
                                stroke,
                                &text,
                                !beyond_len,
                                is_current && !beyond_len,
                            );

                            if !beyond_len && response.double_clicked() {
                                if let Some((idx, _)) = fusion_info {
                                    state.fusion_editing = Some((inst, idx));
                                    fusion_editing_started_this_frame = true;
                                }
                            } else if !beyond_len && response.clicked() && fusion_mode_active {
                                handle_fusion_shift_click(
                                    pattern,
                                    params,
                                    plock,
                                    inst,
                                    global_step,
                                    master_length,
                                    &fusions,
                                    &mut state.fusion_selection_start[inst],
                                );
                            } else if !beyond_len && response.clicked() {
                                if let Some(group) = fusion_group {
                                    toggle_fusion_for_ui(pattern, group, inst);
                                } else {
                                    toggle_step_for_ui(pattern, global_step, inst);
                                }
                                if params.auto_edit.value() {
                                    state.selected_instrument = inst;
                                }
                                state.fusion_selection_start[inst] = None;
                            }

                            if !beyond_len {
                                response.context_menu(|ui| {
                                    if let Some((idx, _)) = fusion_info {
                                        if ui.button("Edit Fusion Steps").clicked() {
                                            state.fusion_editing = Some((inst, idx));
                                            fusion_editing_started_this_frame = true;
                                            ui.close_menu();
                                        }
                                        if ui.button("Delete Fusion").clicked() {
                                            let mut new_fusions = fusions.clone();
                                            if idx < new_fusions.len() {
                                                new_fusions.remove(idx);
                                                pattern.store_fusions(inst, &new_fusions);
                                            }
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                    }
                                    if state.sequencer_mode {
                                        draw_sequencer_plock_menu(
                                            ui,
                                            params,
                                            setter,
                                            inst,
                                            source_step,
                                            state,
                                            fusion_group.is_some(),
                                        );
                                    } else {
                                        draw_plock_menu(
                                            ui,
                                            plock,
                                            sound_settings,
                                            params,
                                            setter,
                                            inst,
                                            source_step,
                                            state,
                                        );
                                    }
                                });
                            }
                        }
                    });

                    draw_param_mini_slider_with_value(
                        ui,
                        setter,
                        hums[inst],
                        0.0,
                        1.0,
                        extra_w,
                        BLUE,
                        "Humanize",
                        |value| format!("{:>3}%", (value * 100.0).round() as i32),
                    );
                    draw_param_mini_slider_with_value(
                        ui,
                        setter,
                        pushes[inst],
                        -50.0,
                        50.0,
                        extra_w,
                        BLUE,
                        "Push/Pull",
                        |value| format!("{:+.0} ms", value),
                    );
                    draw_track_length_control(
                        ui,
                        setter,
                        params,
                        lengths[inst],
                        inst,
                        master_length,
                    );
                });
            }
        });

    let mut fusion_edit_box_rect = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.label(RichText::new("P-Lock Mode").font(f_sans_sb(10.5)).color(INK3));
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
        fusion_edit_box_rect = Some(draw_fusion_edit_box(ui, pattern, state, fusion_mode_active));
    });

    if !fusion_editing_started_this_frame {
        close_fusion_editing_on_outside_click(ui, pattern, state, None, fusion_edit_box_rect);
    }
}

fn draw_page_bar_v2(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    plock: &PlockState,
    state: &mut EditorUIState,
    play_page: usize,
    master_length: usize,
) {
    let page_count = (master_length + 15) / 16;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_sized(
            Vec2::new(84.0, CTL_HEIGHT),
            egui::Label::new(RichText::new("Page").font(f_sans_sb(10.5)).color(INK3)),
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
        }

        let follow = egui::Button::new(
            RichText::new(if state.follow_mode {
                "Follow ON"
            } else {
                "Follow OFF"
            })
            .font(f_sans_sb(11.0))
            .color(if state.follow_mode { Color32::WHITE } else { INK2 }),
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

        ui.add_space((ui.available_width() - 420.0).max(12.0));
        ui.label(RichText::new("Len").font(f_sans_sb(10.5)).color(INK3));
        header_param_slider(ui, setter, &params.pattern_length, 132.0, "", false);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.label(
                RichText::new(format!("{}", master_length))
                    .font(f_mono(12.0))
                    .color(INK),
            );
            ui.label(RichText::new("steps").font(f_sans(9.5)).color(INK3));
        });
        for &len in &[16, 32, 48, 64] {
            let active = master_length == len;
            let btn = egui::Button::new(RichText::new(format!("{}", len)).monospace().size(10.5))
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
    });
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

fn draw_seq_grip_v2(ui: &mut egui::Ui, width: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    // 2x3 dot matrix (drag-handle look; avoids relying on braille glyph coverage)
    let c = rect.center();
    for col in 0..2 {
        for row in 0..3 {
            let p = egui::pos2(c.x + (col as f32 - 0.5) * 4.0, c.y + (row as f32 - 1.0) * 3.0);
            ui.painter().circle_filled(p, 1.0, FAINT);
        }
    }
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
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let stroke = if response.hovered() && enabled {
        egui::Stroke::new(1.0, BLUE)
    } else {
        stroke
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter()
        .rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Inside);
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
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            f_mono_sb(10.0),
            Color32::WHITE,
        );
    }
    response
}

fn step_colors_v2(
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
) -> (Color32, egui::Stroke) {
    if disabled {
        return (
            Color32::from_rgba_unmultiplied(28, 28, 36, 72),
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(58, 58, 72, 72)),
        );
    }
    if selection_start {
        return (Color32::from_rgb(20, 34, 58), egui::Stroke::new(1.5, BLUE));
    }
    if is_fusion_start {
        return (Color32::from_rgb(20, 34, 58), egui::Stroke::new(1.0, BLUE));
    }
    if is_fusion_mid {
        return (Color32::from_rgb(15, 24, 40), egui::Stroke::new(1.0, BLUE_DIM));
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
    text_segmented(ui, "plock_mode", &[("Sound", PL_LINK), ("Sequencer", SEQPL)], selected)
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
        let tw = ui.fonts(|f| f.layout_no_wrap((*label).to_string(), font.clone(), INK).size().x);
        widths.push((tw + padding).max(56.0));
    }
    let total_w: f32 = widths.iter().sum();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, CTL_HEIGHT), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, PANEL2);
    painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, LINE2), egui::StrokeKind::Inside);

    let mut result = selected.min(options.len().saturating_sub(1));
    let mut x = rect.left();
    for (idx, (label, accent)) in options.iter().enumerate() {
        let seg = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(widths[idx], CTL_HEIGHT));
        let active = idx == result;
        let response = ui.interact(seg, ui.make_persistent_id((id_salt, idx)), egui::Sense::click());
        if active {
            painter.rect_filled(seg.shrink(1.0), 5.0, *accent);
        } else if response.hovered() {
            painter.rect_filled(seg.shrink(1.0), 5.0, P_HOVER);
            painter.rect_stroke(seg.shrink(1.0), 5.0, egui::Stroke::new(1.0, *accent), egui::StrokeKind::Inside);
        }
        if idx > 0 {
            painter.line_segment(
                [egui::pos2(seg.left(), rect.top() + 3.0), egui::pos2(seg.left(), rect.bottom() - 3.0)],
                egui::Stroke::new(1.0, LINE2),
            );
        }
        painter.text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            font.clone(),
            if active { Color32::WHITE } else if response.hovered() { INK } else { INK2 },
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
    let label_w = ui.fonts(|f| f.layout_no_wrap(label.to_string(), font.clone(), INK).size().x);
    let total_w = padding_x + led_r * 2.0 + gap + label_w + padding_x;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(total_w, CTL_HEIGHT), egui::Sense::click());
    let hovered = response.hovered();
    let painter = ui.painter_at(rect);

    let fill = if active {
        BLUE_GLOW
    } else {
        PANEL2
    };
    let stroke_color = if active {
        BLUE
    } else if hovered {
        BLUE
    } else {
        LINE2
    };
    let text_color = if active { Color32::WHITE } else if hovered { INK } else { INK2 };

    painter.rect_filled(rect, 6.0, fill);
    painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, stroke_color), egui::StrokeKind::Inside);

    let led_center = egui::pos2(rect.left() + padding_x + led_r, rect.center().y);
    let led_color = if active { BLUE } else { FAINT };
    painter.circle_filled(led_center, led_r, led_color);
    if active {
        painter.circle_filled(led_center, led_r + 2.0, Color32::from_rgba_premultiplied(BLUE.r(), BLUE.g(), BLUE.b(), 45));
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
    painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, LINE2), egui::StrokeKind::Inside);

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
            painter.rect_stroke(seg.shrink(1.0), 5.0, egui::Stroke::new(1.0, BLUE), egui::StrokeKind::Inside);
        }
        if idx == 1 {
            painter.line_segment(
                [egui::pos2(seg.left(), rect.top() + 3.0), egui::pos2(seg.left(), rect.bottom() - 3.0)],
                egui::Stroke::new(1.0, LINE2),
            );
        }
        painter.text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            *label,
            f_sans_sb(10.5),
            if active { Color32::WHITE } else if response.hovered() { INK } else { INK2 },
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
                    ui.label(RichText::new(note_name(note_val)).font(f_mono_sb(13.0)).color(INK));
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
// Sound Panel (always visible, tabbed by instrument)
// ---------------------------------------------------------------------------------------------------------------
fn draw_sound_panel(
    ui: &mut egui::Ui,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    setter: &ParamSetter,
    state: &mut EditorUIState,
) {
    state.selected_instrument = state.selected_instrument.min(DrumVoice::COUNT - 1);

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
    ui.allocate_ui_at_rect(header_rect.shrink2(Vec2::new(14.0, 0.0)), |ui| {
        ui.horizontal_centered(|ui| {
            ui.set_height(header_rect.height());
            ui.label(
                RichText::new("Sound Editor")
                    .font(f_sans_bold(13.0))
                    .color(Color32::WHITE),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(
                    crate::instrument_registry::INSTRUMENTS[state.selected_instrument].name,
                )
                .font(f_mono(11.0))
                .color(INK3),
            );
            // (Engine selector belongs to the future modular phase — omitted for now.)
        });
    });

    let tabs_rect = ui
        .allocate_exact_size(Vec2::new(ui.available_width(), 45.0), egui::Sense::hover())
        .0;
    ui.painter().rect_filled(tabs_rect, 0.0, PANEL);
    ui.painter().hline(
        tabs_rect.x_range(),
        tabs_rect.bottom(),
        egui::Stroke::new(1.0, LINE),
    );
    ui.allocate_ui_at_rect(tabs_rect.shrink2(Vec2::new(12.0, 9.0)), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP_TIGHT;
            let tab_count = crate::instrument_registry::INSTRUMENTS.len().max(1);
            let tab_w = ((tabs_rect.width() - 24.0 - GAP_TIGHT * (tab_count - 1) as f32)
                / tab_count as f32)
                .floor();
            for (i, inst_def) in crate::instrument_registry::INSTRUMENTS.iter().enumerate() {
                let selected = state.selected_instrument == i;
                let btn = egui::Button::new(
                    RichText::new(inst_def.label)
                        .monospace()
                        .size(10.5)
                        .color(if selected { Color32::WHITE } else { INK2 }),
                )
                .min_size(Vec2::new(tab_w, CTL_HEIGHT))
                .fill(if selected { BLUE } else { PANEL2 })
                .stroke(egui::Stroke::new(1.0, if selected { BLUE } else { LINE2 }))
                .corner_radius(6.0);
                if ui.add(btn).on_hover_text(inst_def.full_name).clicked() {
                    state.selected_instrument = i;
                }
            }
        });
    });

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

            // ------ Dev Tools: Preset Dumps ------
            if cfg!(debug_assertions) {
                ui.collapsing("Dev: Preset Dumps", |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut state.dump_name_input);
                if ui.button("Dump").clicked() {
                    let instrument =
                        &crate::instrument_registry::INSTRUMENTS[state.selected_instrument];
                    let mut specials = Vec::new();
                    for def in instrument.special_params {
                        if let Some(param) =
                            params.special_param(state.selected_instrument, def.special_index)
                        {
                            specials.push(param.value());
                        } else {
                            specials.push(0.0);
                        }
                    }
                    let algo = params.algos()[state.selected_instrument].value() as u8;
                    // Skip Analog for instruments that don't use it
                    let standards = if matches!(state.selected_instrument, 2 | 3 | 7 | 8 | 10 | 12)
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
                                state.selected_instrument = dump.instrument_idx;
                                let target_inst = &sound_settings.instruments[dump.instrument_idx];
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
                                    dump.instrument_idx,
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
                                let algo_param = params.algos()[dump.instrument_idx];
                                setter.set_parameter(algo_param, dump.algo as i32);
                                let inst_def =
                                    &crate::instrument_registry::INSTRUMENTS[dump.instrument_idx];
                                for (i, def) in inst_def.special_params.iter().enumerate() {
                                    if let Some(param) =
                                        params.special_param(dump.instrument_idx, def.special_index)
                                    {
                                        if i < dump.specials.len() {
                                            setter.set_parameter(param, dump.specials[i]);
                                        }
                                    }
                                }
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

            // Data-driven grouped Sound Panel
            let instrument = &crate::instrument_registry::INSTRUMENTS[state.selected_instrument];
            let standard_defs = instrument.standard_params;
            let special_defs = instrument.special_params;

            for family in [
                crate::instrument_registry::ParamFamily::Osc,
                crate::instrument_registry::ParamFamily::Env,
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
                                    let ft = crate::instrument_registry::filter_type_label(state.selected_instrument);
                                    format!("{} ({})", def.label, ft)
                                } else {
                                    def.label.to_string()
                                };

                                let is_bass_drum = state.selected_instrument == 0 || state.selected_instrument == 11;
                                let freq_in_notes = if is_bass_drum && def.field == crate::instrument_registry::StandardField::Freq {
                                    let freq_mode_param = if state.selected_instrument == 0 {
                                        &params.freq_mode_kick
                                    } else {
                                        &params.freq_mode_bassdrum808
                                    };
                                    freq_mode_param.value()
                                } else {
                                    false
                                };

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
                                            let freq_mode_param = if state.selected_instrument == 0 {
                                                &params.freq_mode_kick
                                            } else {
                                                &params.freq_mode_bassdrum808
                                            };
                                            setter.set_parameter(freq_mode_param, new_mode);
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

                    // Special params for this family
                    for def in special_defs.iter().filter(|d| d.family == family) {
                        if let Some(param) = params.special_param(state.selected_instrument, def.special_index) {
                            ui.horizontal(|ui| {
                                // Boolean toggle for on/off switches (min=0, max=1)
                                if def.min == 0.0 && def.max == 1.0 && def.label.to_lowercase().contains("pre-filter") {
                                    let mut value = param.value();
                                    if draw_editor_switch_row(ui, def.label, &mut value).changed() {
                                        setter.set_parameter(param, value);
                                    }
                                // Saturation Type: show select with names instead of number slider
                                } else if def.label.to_lowercase().contains("saturation type") {
                                    let type_names = ["None", "SoftClip", "Valve", "Transistor", "HardClip", "Tape"];
                                    let current_idx = (param.value() as usize).min(type_names.len().saturating_sub(1));
                                    editor_label(ui, def.label);
                                    if let (_, Some(idx)) = styled_select(ui, def.name, current_idx, &type_names, 146.0) {
                                        setter.set_parameter(param, idx as f32);
                                    }
                                // Cymbal Noise Type: show select with names
                                } else if def.label.to_lowercase().contains("noise type") {
                                    let type_names = ["White", "Pink", "Brown", "Blue"];
                                    let current_idx = (param.value() as usize).min(type_names.len().saturating_sub(1));
                                    editor_label(ui, def.label);
                                    if let (_, Some(idx)) = styled_select(ui, def.name, current_idx, &type_names, 146.0) {
                                        setter.set_parameter(param, idx as f32);
                                    }
                                // Kick Click Type: show select with names
                                } else if def.label.to_lowercase().contains("click type") {
                                    let type_names = ["Soft", "Medium", "Hard"];
                                    let current_idx = (param.value() as usize).min(type_names.len().saturating_sub(1));
                                    editor_label(ui, def.label);
                                    if let (_, Some(idx)) = styled_select(ui, def.name, current_idx, &type_names, 146.0) {
                                        setter.set_parameter(param, idx as f32);
                                    }
                                } else {
                                    let mut value = param.value();
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
                                        setter.set_parameter(param, value);
                                    }
                                }
                            });
                        }
                    }

                    // Algorithm selector inside OSC family
                    if family == crate::instrument_registry::ParamFamily::Osc {
                        if let Some(voice) = DrumVoice::from_index(state.selected_instrument) {
                            let algos = synthesis::algos_for(voice);
                            if algos.len() > 1 && state.selected_instrument != 3 {
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

fn draw_fusion_edit_box(
    ui: &mut egui::Ui,
    pattern_for_ui: &SharedPattern,
    state: &mut EditorUIState,
    fusion_mode_active: bool,
) -> egui::Rect {
    let box_size = Vec2::new(380.0, 28.0);

    ui.allocate_ui_with_layout(
        box_size,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(24, 24, 30))
                .stroke(egui::Stroke::new(1.0, LINE2))
                .corner_radius(5.0)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(box_size.x - 8.0, box_size.y - 8.0));
                    ui.horizontal(|ui| {
                        if let Some((instrument, index, group)) =
                            edited_fusion_for_ui(pattern_for_ui, state)
                        {
                            ui.label(
                                RichText::new(format!(
                                    "Fusion {}-{} (cells)",
                                    group.start_cell + 1,
                                    group.end_cell + 1
                                ))
                                .strong()
                                .size(11.0),
                            );
                            ui.label(RichText::new("Steps:").size(11.0));

                            let mut step_count = group.step_count as i32;
                            if ui
                                .add_sized(
                                    Vec2::new(48.0, 18.0),
                                    egui::DragValue::new(&mut step_count)
                                        .speed(1.0)
                                        .range(1..=64),
                                )
                                .changed()
                            {
                                let mut new_fusions = pattern_for_ui.load_fusions(instrument);
                                if let Some(group) = new_fusions.get_mut(index) {
                                    group.step_count = step_count as u8;
                                    pattern_for_ui.store_fusions(instrument, &new_fusions);
                                }
                            }

                            if ui.button("Delete").clicked() {
                                let mut new_fusions = pattern_for_ui.load_fusions(instrument);
                                if index < new_fusions.len() {
                                    new_fusions.remove(index);
                                    pattern_for_ui.store_fusions(instrument, &new_fusions);
                                }
                                state.fusion_editing = None;
                            }
                            if ui.button("Close").clicked() {
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
    )
    .response
    .rect
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

fn mixer_rows(params: &DrumFlashParams) -> [MixerRow<'_>; DrumVoice::COUNT] {
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
        egui::Label::new(
            RichText::new(label)
                .font(f_mono_med(9.5))
                .color(INK3),
        ),
    );
}

fn chip_button(ui: &mut egui::Ui, label: &str, accent: bool, color: Color32) -> egui::Response {
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
        egui::Button::new(RichText::new(label).size(10.5).color(text_color).font(f_sans_sb(11.0)))
            .min_size(Vec2::new(0.0, CTL_HEIGHT))
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(6.0),
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
    bpm: f32,
    pattern_length: usize,
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

    midi_export::export_pattern_to_midi(pattern, bpm, pattern_length, &path)?;
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
    setter: &ParamSetter,
    instrument: usize,
    step: usize,
    state: &mut EditorUIState,
) {
    use crate::plock::{FIELD_COUNT, SPECIAL_FIELD_START};

    ui.label(
        RichText::new(format!(
            "Plock {} --- Step {}",
            crate::instrument_registry::INSTRUMENTS[instrument].label,
            step + 1
        ))
        .strong(),
    );
    ui.separator();

    let inst = &sound_settings.instruments[instrument];
    let global = inst.load();

    let has_plock = plock.masks.is_active(instrument, step);

    // ------ Creation ------
    if !has_plock {
        ui.label("Create plock:");
        if ui.button("Link to global").clicked() {
            plock.masks.set_active(instrument, step, true);
        }
        if ui.button("Snapshot current settings").clicked() {
            let mut special = [0.0f32; 32];
            for def in crate::instrument_registry::special_params(instrument) {
                if def.special_index < special.len() {
                    special[def.special_index] = params
                        .special_param(instrument, def.special_index)
                        .map(|param| param.value())
                        .unwrap_or(def.default);
                }
            }
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
        // Paste plock from clipboard (even if no plock exists here yet)
        if let Some(ref entry) = state.plock_clipboard {
            if entry.instrument == instrument {
                if ui.button("Paste Plock").clicked() {
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
        "Linked to global"
    } else if mask == all_bits {
        "Full snapshot"
    } else {
        "Mixed"
    };
    ui.label(RichText::new(mode_text).small());
    ui.separator();

    // ------ Volume (most used, shown first) ------
    {
        let vol_field = crate::instrument_registry::StandardField::Volume.plock_field_index();
        let mut vol_value = if plock.field_masks.is_set(instrument, step, vol_field) {
            plock.values.get(instrument, step, vol_field)
        } else {
            global.2
        };
        let overridden = plock.field_masks.is_set(instrument, step, vol_field);
        let label_text = if overridden {
            RichText::new("Volume").strong()
        } else {
            RichText::new("Volume").weak()
        };
        let (changed, reset) = ui
            .horizontal(|ui| {
                ui.label(label_text);
                let slider = LocalParamSlider::new(&mut vol_value, 0.0..=2.0).with_width(120.0);
                let response = ui.add(slider);
                let c = response.changed();
                let r = overridden && ui.small_button("Undo").clicked();
                (c, r)
            })
            .inner;
        if changed {
            plock.set_field(instrument, step, vol_field, vol_value);
        }
        if reset {
            plock.field_masks.clear(instrument, step, vol_field);
        }
    }
    ui.separator();

    // ------ Helpers ------
    let draw_slider = |ui: &mut egui::Ui,
                       label: &str,
                       value: &mut f32,
                       range: std::ops::RangeInclusive<f32>,
                       log: bool,
                       field: usize| {
        let overridden = plock.field_masks.is_set(instrument, step, field);
        let label_text = if overridden {
            RichText::new(label).strong()
        } else {
            RichText::new(label).weak()
        };
        let (changed, reset) = ui
            .horizontal(|ui| {
                ui.label(label_text);
                let slider = LocalParamSlider::new(value, range.clone())
                    .logarithmic(log)
                    .with_width(120.0);
                let response = ui.add(slider);
                let c = response.changed();
                let r = overridden && ui.small_button("Undo").clicked();
                (c, r)
            })
            .inner;
        if changed {
            plock.set_field(instrument, step, field, *value);
        }
        if reset {
            plock.field_masks.clear(instrument, step, field);
        }
    };

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

    let is_bass_drum_plock = instrument == 0 || instrument == 11;
    let freq_in_notes_plock = if is_bass_drum_plock {
        let freq_mode_param = if instrument == 0 {
            &params.freq_mode_kick
        } else {
            &params.freq_mode_bassdrum808
        };
        freq_mode_param.value()
    } else {
        false
    };

    // Note display toggle for bass drums in plock
    if is_bass_drum_plock {
        let freq_mode_param = if instrument == 0 {
            &params.freq_mode_kick
        } else {
            &params.freq_mode_bassdrum808
        };
        let mut freq_in_notes = freq_mode_param.value();
        if ui.checkbox(&mut freq_in_notes, "Notes").changed() {
            setter.set_parameter(freq_mode_param, freq_in_notes);
        }
    }

    // Data-driven standard params
    let inst_def = &crate::instrument_registry::INSTRUMENTS[instrument];
    for def in inst_def.standard_params {
        // Volume is already shown at the top of the menu
        if def.field == crate::instrument_registry::StandardField::Volume {
            continue;
        }
        let field_index = def.field.plock_field_index();
        let mut value = if plock.field_masks.is_set(instrument, step, field_index) {
            plock.values.get(instrument, step, field_index)
        } else {
            get_global_value(def.field)
        };

        // Special case: frequency in note mode for bass drums
        if def.field == crate::instrument_registry::StandardField::Freq && freq_in_notes_plock {
            let ratio = inst_def.freq_display_ratio;
            let overridden = plock.field_masks.is_set(instrument, step, field_index);
            let note_val = freq_to_note(value * ratio).round();
            let label_text = if overridden {
                RichText::new(format!("{}: {}", def.label, note_name(note_val))).strong()
            } else {
                RichText::new(format!("{}: {}", def.label, note_name(note_val))).weak()
            };
            ui.horizontal(|ui| {
                ui.label(label_text);
                if ui.button("-").clicked() {
                    let new_note = (note_val - 1.0).max(0.0);
                    value = note_to_freq(new_note) / ratio;
                    plock.set_field(instrument, step, field_index, value);
                }
                if ui.button("+").clicked() {
                    let new_note = (note_val + 1.0).min(127.0);
                    value = note_to_freq(new_note) / ratio;
                    plock.set_field(instrument, step, field_index, value);
                }
                if overridden && ui.small_button("Undo").clicked() {
                    plock.field_masks.clear(instrument, step, field_index);
                }
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
                draw_slider(
                    ui,
                    def.label,
                    &mut value,
                    *min..=*max,
                    *logarithmic,
                    field_index,
                );
            }
            crate::instrument_registry::ParamWidget::Checkbox => {
                let overridden = plock.field_masks.is_set(instrument, step, field_index);
                let label_text = if overridden {
                    RichText::new(def.label).strong()
                } else {
                    RichText::new(def.label).weak()
                };
                let (changed, reset) = ui
                    .horizontal(|ui| {
                        ui.label(label_text);
                        let mut checked = value >= 0.5;
                        let c = ui.add(egui::Checkbox::new(&mut checked, "")).changed();
                        if c {
                            value = if checked { 1.0 } else { 0.0 };
                        }
                        let r = overridden && ui.small_button("Undo").clicked();
                        (c, r)
                    })
                    .inner;
                if changed {
                    plock.set_field(instrument, step, field_index, value);
                }
                if reset {
                    plock.field_masks.clear(instrument, step, field_index);
                }
            }
        }
    }

    // ------ Algo ------
    let mut algo_val = if plock.field_masks.is_set(instrument, step, 13) {
        plock.values.get(instrument, step, 13) as u8
    } else {
        params.algos()[instrument].value() as u8
    };
    let algo_overridden = plock.field_masks.is_set(instrument, step, 13);
    let algo_label = if algo_overridden {
        RichText::new("Algo").strong()
    } else {
        RichText::new("Algo").weak()
    };
    let mut algo_val_f32 = algo_val as f32;
    let (algo_changed, algo_reset) = ui
        .horizontal(|ui| {
            ui.label(algo_label);
            let slider = LocalParamSlider::new(&mut algo_val_f32, 0.0..=3.0).with_width(120.0);
            let c = ui.add(slider).changed();
            let r = algo_overridden && ui.small_button("Undo").clicked();
            (c, r)
        })
        .inner;
    if algo_changed {
        algo_val = algo_val_f32.round() as u8;
    }
    if algo_changed {
        plock.set_field(instrument, step, 13, algo_val as f32);
    }
    if algo_reset {
        plock.field_masks.clear(instrument, step, 13);
    }

    // ------ Special params ------
    let special_defs = crate::instrument_registry::special_params(instrument);
    for def in special_defs {
        if def.special_index >= 8 {
            continue;
        }
        let field = SPECIAL_FIELD_START + def.special_index;
        let mut value = if plock.field_masks.is_set(instrument, step, field) {
            plock.values.get(instrument, step, field)
        } else {
            params
                .special_param(instrument, def.special_index)
                .map(|p| p.value())
                .unwrap_or(def.default)
        };
        let overridden = plock.field_masks.is_set(instrument, step, field);
        let label_text = if overridden {
            RichText::new(def.label).strong()
        } else {
            RichText::new(def.label).weak()
        };
        let (changed, reset) = ui
            .horizontal(|ui| {
                ui.label(label_text);
                let log = def.min > 0.0 && def.max / def.min >= 20.0;
                let slider = LocalParamSlider::new(&mut value, def.min..=def.max)
                    .logarithmic(log)
                    .with_width(120.0);
                let c = ui.add(slider).changed();
                let r = overridden && ui.small_button("Undo").clicked();
                (c, r)
            })
            .inner;
        if changed {
            plock.set_field(instrument, step, field, value);
        }
        if reset {
            plock.field_masks.clear(instrument, step, field);
        }
    }

    ui.separator();
    if ui.button("Copy Plock").clicked() {
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
            if ui.button("Paste Plock").clicked() {
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
    if ui.button("Clear plock").clicked() {
        plock.clear(instrument, step);
    }
}

fn draw_sequencer_plock_menu(
    ui: &mut egui::Ui,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    step: usize,
    _state: &mut EditorUIState,
    stutter_disabled: bool,
) {
    use crate::plock::{SequencerStepParams, StepCondition};

    ui.label(
        RichText::new(format!(
            "Seq Plock {} --- Step {}",
            crate::instrument_registry::INSTRUMENTS[instrument].label,
            step + 1
        ))
        .strong(),
    );
    ui.separator();

    let seq_plock = &params.seq_plock_state.state;
    let has_seq_plock = seq_plock.is_active(instrument, step);
    let current = seq_plock.get(instrument, step).unwrap_or_default();
    let mut changed_this_frame = false;

    // Probability slider
    let mut prob = current.probability;
    ui.label(format!("Probability: {:.0}%", prob * 100.0));
    if ui
        .add(egui::Slider::new(&mut prob, 0.0..=1.0).show_value(false))
        .changed()
    {
        seq_plock.set_probability(instrument, step, prob);
        changed_this_frame = true;
    }

    // Stutter count (1-16). Disabled on fused cells because fusion pulses are
    // their own timing feature and should not stack with seq-plock stutter.
    if stutter_disabled {
        if has_seq_plock && current.stutter_count != 1 {
            let mut fixed = current;
            fixed.stutter_count = 1;
            seq_plock.set(instrument, step, &fixed);
        }
        ui.label(
            RichText::new("Stutter: disabled on fusion")
                .size(11.0)
                .color(INK2),
        );
    } else {
        let mut stutter = current.stutter_count.max(1) as i32;
        ui.label(format!("Stutter: {}x", stutter));
        if ui.add(egui::Slider::new(&mut stutter, 1..=16)).changed() {
            seq_plock.set_stutter(instrument, step, stutter as u8);
            changed_this_frame = true;
        }
    }

    // Condition selector. Avoid egui::ComboBox here: nested popups inside
    // context_menu can close before the selected value is committed.
    let all_conditions = StepCondition::all();
    ui.label("Condition:");
    egui::Grid::new(format!("condition_grid_{}_{}", instrument, step))
        .num_columns(3)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for (idx, cond) in all_conditions.iter().copied().enumerate() {
                if ui
                    .selectable_label(current.condition == cond, cond.label())
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

    // Clear button
    ui.separator();
    if has_seq_plock || changed_this_frame {
        if ui.button("Clear Seq Plock").clicked() {
            seq_plock.clear(instrument, step);
        }
    } else {
        ui.label("No sequencer plock");
        if ui.button("Create Seq Plock").clicked() {
            seq_plock.set(instrument, step, &SequencerStepParams::default());
        }
    }
}
