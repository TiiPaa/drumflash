use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32, RichText, Vec2},
    resizable_window::ResizableWindow,
    widgets,
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
    generator, midi_export,
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
mod schema;

use design_system::*;
use envelope_viz::{draw_amp_envelope, draw_filter_envelope};
use local_param_slider::LocalParamSlider;
use schema::{category_for_instrument, instrument_label, instrument_name, Category};

// ---------------------------------------------------------------------------------------------------------------
// Frequency / Note conversion utilities
// ---------------------------------------------------------------------------------------------------------------
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

const VOLUME_DB_MIN: f32 = -60.0;
const VOLUME_DB_MAX: f32 = 6.0;

fn gain_to_volume_db(gain: f32) -> f32 {
    if gain <= 0.0 {
        VOLUME_DB_MIN
    } else {
        (20.0 * gain.log10()).clamp(VOLUME_DB_MIN, VOLUME_DB_MAX)
    }
}

fn volume_db_to_gain(db: f32) -> f32 {
    let db = db.clamp(VOLUME_DB_MIN, VOLUME_DB_MAX);
    if db <= VOLUME_DB_MIN {
        0.0
    } else if db >= VOLUME_DB_MAX {
        2.0
    } else {
        10.0f32.powf(db / 20.0).clamp(0.0, 2.0)
    }
}

fn format_volume_db(gain: f32) -> String {
    if gain <= 0.0 {
        "-inf dB".to_string()
    } else {
        format!("{:.1} dB", 20.0 * gain.log10())
    }
}

fn draw_volume_db_slider(
    ui: &mut egui::Ui,
    gain: &mut f32,
    width: f32,
    draw_value: bool,
) -> egui::Response {
    let mut db = gain_to_volume_db(*gain);
    let response = ui.add(
        LocalParamSlider::new(&mut db, VOLUME_DB_MIN..=VOLUME_DB_MAX)
            .with_width(width)
            .without_value()
            .reset_value(0.0),
    );
    if response.changed() {
        *gain = volume_db_to_gain(db);
    }
    let response = response.on_hover_text(format!("Volume {}", format_volume_db(*gain)));
    if draw_value {
        ui.label(
            RichText::new(format_volume_db(*gain))
                .monospace()
                .size(11.0),
        );
    }
    response
}

fn draw_track_length_control(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    length_param: &IntParam,
    instrument: usize,
    master_length: usize,
) {
    let follows_pattern = !params.track_length_overrides.is_overridden(instrument);
    let mut length_value = if follows_pattern {
        master_length as i32
    } else {
        length_param.value()
    };

    let response = ui.add_sized(
        Vec2::new(35.0, 20.0),
        egui::DragValue::new(&mut length_value)
            .speed(1.0)
            .range(1..=64),
    );
    let changed = response.changed();
    let response = response.on_hover_text(if follows_pattern {
        "Follows pattern length. Drag to override this lane."
    } else {
        "Manual lane length. Right-click to follow pattern length."
    });

    response.context_menu(|ui| {
        if follows_pattern {
            ui.label("Already follows pattern length");
        } else if ui.button("Follow pattern length").clicked() {
            params
                .track_length_overrides
                .set_overridden(instrument, false);
            setter.set_parameter(length_param, master_length as i32);
            ui.close_menu();
        }
    });

    if changed {
        params
            .track_length_overrides
            .set_overridden(instrument, true);
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
        |_egui_ctx, _state| {},
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

                    ui.separator();

                    // --- Layout 2 colonnes avec largeurs fixes ---
                    let left_w = 900.0;
                    let right_w = 560.0;
                    let gap = 20.0;
                    let content_h = ui.available_height();

                    ui.horizontal_top(|ui| {
                        // Colonne gauche
                        ui.allocate_ui_with_layout(
                            Vec2::new(left_w, 0.0),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                draw_grid(
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
                                ui.separator();
                                draw_pattern_bank(
                                    ui,
                                    state,
                                    &params_for_ui,
                                    &pattern_for_ui,
                                    &save_pattern_request,
                                    &load_pattern_request,
                                    &clear_plocks_request_for_ui,
                                );
                                ui.separator();
                                if params_for_ui.song_mode.value() {
                                    draw_song_editor(
                                        ui,
                                        setter,
                                        &params_for_ui,
                                        state,
                                        &song_mode_for_ui,
                                        &song_position_for_ui,
                                    );
                                } else {
                                    draw_generator_panel(
                                        ui,
                                        setter,
                                        &params_for_ui,
                                        &pattern_for_ui,
                                        state,
                                    );
                                }
                            },
                        );

                        ui.add_space(gap);

                        // Colonne droite
                        ui.allocate_ui_with_layout(
                            Vec2::new(right_w, content_h),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                draw_sound_panel(
                                    ui,
                                    &sound_settings_for_ui,
                                    &params_for_ui,
                                    setter,
                                    state,
                                );
                            },
                        );
                    });
                });
        },
    )
}

// ---------------------------------------------------------------------------------------------------------------
// Header bar: Brand + Play + BPM + Sliders + Toggles
// ---------------------------------------------------------------------------------------------------------------
fn draw_header_bar(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    state: &mut EditorUIState,
    _save_pattern_request: &Arc<AtomicU32>,
    _load_pattern_request: &Arc<AtomicU32>,
    _song_mode: &Arc<AtomicBool>,
    _song_position: &Arc<AtomicU32>,
) {
    ui.horizontal(|ui| {
        // Brand
        ui.horizontal(|ui| {
            ui.label(RichText::new("FLASH DRUM").strong().size(15.0));
            ui.label(
                RichText::new(format!("v{} · {}", env!("CARGO_PKG_VERSION"), BUILD_ID))
                    .monospace()
                    .size(10.0)
                    .color(Color32::from_rgb(100, 100, 110)),
            );
        });

        ui.separator();

        // Sliders
        ui.add(widgets::ParamSlider::for_param(&params.master_volume, setter).with_width(80.0));
        ui.add(widgets::ParamSlider::for_param(&params.swing, setter).with_width(80.0));
        enum_combo(ui, setter, &params.groove_type, "");

        ui.separator();

        // Toggles
        bool_checkbox(ui, setter, &params.use_internal_sequencer, "Seq");
        bool_checkbox(ui, setter, &params.hihat_chokes_oh, "Choke");
        bool_checkbox(ui, setter, &params.auto_edit, "Auto-Edit");
        bool_checkbox(ui, setter, &params.song_mode, "Song");
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
        ui.label(RichText::new("Patterns").strong().size(12.0));
        ui.label(
            RichText::new(format!("[P:{} S:{}]", sound_plock_count, seq_plock_count))
                .size(9.0)
                .monospace()
                .color(Color32::from_rgb(100, 100, 110)),
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
            Color32::from_rgb(48, 48, 58)
        };
        let save_btn = egui::Button::new(RichText::new("Save").size(10.0).strong().monospace())
            .min_size(Vec2::new(44.0, 26.0))
            .fill(save_fill)
            .stroke(egui::Stroke::new(
                1.5,
                if is_save_mode {
                    Color32::from_rgb(120, 200, 255)
                } else {
                    Color32::from_rgb(58, 58, 72)
                },
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

            let btn_size = Vec2::new(36.0, 26.0);
            let fill = if is_loaded {
                Color32::from_rgb(40, 60, 40)
            } else if occupied {
                Color32::from_rgb(48, 48, 58)
            } else {
                Color32::from_rgb(16, 16, 22) // much darker for empty slot
            };
            let stroke_color = if is_loaded {
                Color32::from_rgb(100, 220, 120) // green ring for loaded
            } else if occupied {
                Color32::from_rgb(100, 100, 120)
            } else {
                Color32::from_rgb(40, 40, 50) // dimmer border for empty slot
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

                        let label_color = if is_loaded {
                            Color32::from_rgb(150, 255, 150)
                        } else {
                            Color32::from_rgb(200, 200, 210)
                        };
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
            Color32::from_rgb(48, 48, 58)
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
                Color32::from_rgb(58, 58, 72)
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
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Generator panel (preset chips + generator controls + GENERATE button)
// ---------------------------------------------------------------------------------------------------------------
fn draw_generator_panel(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
) {
    ui.label(RichText::new("Generator").strong());
    draw_preset_bar(ui, pattern, params, setter, state);
    draw_generator_bar(ui, setter, params, pattern, state);
}

// ---------------------------------------------------------------------------------------------------------------
// Song editor: sequence of pattern slots (P1-P8)
// ---------------------------------------------------------------------------------------------------------------
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
                    Color32::from_rgb(36, 36, 44)
                })
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(58, 58, 72)))
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
                            Color32::from_rgb(48, 48, 58)
                        } else {
                            Color32::from_rgb(28, 28, 36)
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if is_current {
                                Color32::from_rgb(255, 150, 150)
                            } else {
                                Color32::from_rgb(58, 58, 72)
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
// Top bar: Master Volume / Swing / Groove / Choke
// ---------------------------------------------------------------------------------------------------------------
fn draw_top_bar(ui: &mut egui::Ui, setter: &ParamSetter, params: &DrumFlashParams) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Vol").strong());
        ui.add(widgets::ParamSlider::for_param(&params.master_volume, setter).with_width(80.0));

        ui.add_space(16.0);
        ui.label(RichText::new("Swing").strong());
        ui.add(widgets::ParamSlider::for_param(&params.swing, setter).with_width(80.0));

        ui.add_space(8.0);
        enum_combo(ui, setter, &params.groove_type, "Groove");

        ui.add_space(16.0);
        ui.label(RichText::new("Len").strong());
        ui.add(widgets::ParamSlider::for_param(&params.pattern_length, setter).with_width(60.0));

        ui.add_space(16.0);
        bool_checkbox(ui, setter, &params.hihat_chokes_oh, "Choke");

        ui.add_space(16.0);
        bool_checkbox(ui, setter, &params.auto_edit, "Auto Edit");
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Song mode placeholder: P1..P8 slots
// ---------------------------------------------------------------------------------------------------------------
fn draw_song_bar(ui: &mut egui::Ui, state: &mut EditorUIState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Song").strong());
        ui.add_space(4.0);
        if ui.button("--").clicked() && state.selected_pattern_slot > 0 {
            state.selected_pattern_slot -= 1;
        }
        for i in 0..8 {
            let selected = state.selected_pattern_slot == i;
            let text = format!("P{}", i + 1);
            let btn = egui::Button::new(RichText::new(text).size(11.0))
                .min_size(Vec2::new(28.0, 22.0))
                .fill(if selected {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(36, 36, 36)
                });
            if ui.add(btn).clicked() {
                state.selected_pattern_slot = i;
            }
        }
        if ui.button("-").clicked() && state.selected_pattern_slot < 7 {
            state.selected_pattern_slot += 1;
        }
    });
}

// ---------------------------------------------------------------------------------------------------------------
// Presets / Random / Export MIDI
// ---------------------------------------------------------------------------------------------------------------
fn draw_preset_bar(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    state: &mut EditorUIState,
) {
    ui.horizontal(|ui| {
        // Presets (left)
        ui.label(RichText::new("Presets").strong().size(11.0));
        let pattern_length = params.pattern_length.value() as usize;
        if ui.button("Rock").clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::rock_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        if ui.button("Funk").clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::funk_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        if ui.button("Disco").clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::disco_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        ui.add_space(16.0);

        // Random (middle)
        if ui.button("Random").clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::random_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }

        ui.add_space(16.0);

        // Export MIDI (right)
        ui.label(RichText::new("Export").strong().size(11.0));
        let export_btn = egui::Button::new("MIDI");
        let response = ui.add(export_btn);
        if response.clicked() {
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
        response.on_hover_text("Export MIDI file to Documents/Flash Drum/exports");

        let drag_btn = egui::Button::new("Drag").sense(egui::Sense::click_and_drag());
        let drag_response = ui.add(drag_btn);
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
        drag_response.on_hover_text("Open external MIDI drag handle");

        if let Some(path) = &state.last_midi_export_path {
            if ui.button("Copy Path").clicked() {
                ui.ctx().copy_text(path.clone());
            }
            ui.label(RichText::new("Exported").size(10.0));
        } else if state.last_midi_export_error.is_some() {
            ui.label(
                RichText::new("Export failed")
                    .size(10.0)
                    .color(Color32::RED),
            );
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
    ui.horizontal(|ui| {
        ui.label(RichText::new("Generator").strong().size(11.0));
        enum_combo(ui, setter, &params.generator_type, "Type");
        enum_combo(ui, setter, &params.style_primary, "A");
        enum_combo(ui, setter, &params.style_secondary, "B");
        ui.label("Mix");
        ui.add(widgets::ParamSlider::for_param(&params.style_mix, setter).with_width(50.0));
        ui.label("Dens");
        ui.add(widgets::ParamSlider::for_param(&params.gen_density, setter).with_width(50.0));
        ui.label("Var");
        ui.add(widgets::ParamSlider::for_param(&params.gen_variation, setter).with_width(50.0));

        let gen_btn = egui::Button::new(RichText::new(" GENERATE ").strong().size(13.0))
            .fill(Color32::from_rgb(56, 132, 255));
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
}

// ---------------------------------------------------------------------------------------------------------------
// Pattern grid with per-row Hum/Push/Len
// ---------------------------------------------------------------------------------------------------------------
fn draw_grid(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    voice_test_triggers: &[AtomicBool; DrumVoice::COUNT],
    current_step: &AtomicU32,
    current_steps: &[AtomicU32; DrumVoice::COUNT],
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
) {
    let mixer_rows = mixer_rows(params);
    let hums: [&FloatParam; DrumVoice::COUNT] = std::array::from_fn(|i| params.humanizes()[i]);
    let pushes: [&FloatParam; DrumVoice::COUNT] = std::array::from_fn(|i| params.pushes()[i]);
    let lengths: [&IntParam; DrumVoice::COUNT] = std::array::from_fn(|i| params.lengths()[i]);

    // Page navigation + Follow toggle
    let play_page = {
        let step = current_step.load(Ordering::Relaxed) as usize;
        step / 16
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Page:").strong());
        for page in 0..4 {
            let is_active = state.current_page == page;
            let btn = egui::Button::new(format!("{}", page + 1))
                .min_size(Vec2::new(28.0, 22.0))
                .fill(if is_active {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(40, 40, 40)
                });
            let response = ui.add(btn);
            // LED rouge sous le bouton de la page en cours de lecture
            if play_page == page && play_page < 4 {
                let led_center = response.rect.center_bottom() + egui::vec2(0.0, 4.0);
                ui.painter()
                    .circle_filled(led_center, 3.0, Color32::from_rgb(255, 40, 40));
            }
            if response.clicked() {
                state.current_page = page;
            }
            response.context_menu(|ui| {
                if ui.button("Copy Page").clicked() {
                    let base = page * 16;
                    let mut triggers = [0u16; 16];
                    let mut plocks = Vec::new();
                    let mut fusions = Vec::new();
                    for i in 0..16 {
                        let step = base + i;
                        triggers[i] = pattern.load_step_mask(step);
                        for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
                            if plock.masks.is_active(inst, step) {
                                let field_mask = plock.field_masks.get_raw(inst, step);
                                let mut values = Vec::with_capacity(crate::plock::FIELD_COUNT);
                                for field in 0..crate::plock::FIELD_COUNT {
                                    values.push(plock.values.get(inst, step, field));
                                }
                                plocks.push(PlockClipboardEntry {
                                    instrument: inst,
                                    step: i,
                                    field_mask,
                                    values,
                                });
                            }
                        }
                    }
                    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
                        for group in pattern.load_fusions(inst) {
                            let start = group.start_cell as usize;
                            let end = group.end_cell as usize;
                            if start >= base && end < base + 16 {
                                fusions.push(FusionClipboardEntry {
                                    instrument: inst,
                                    start_step: start - base,
                                    end_step: end - base,
                                    step_count: group.step_count,
                                });
                            }
                        }
                    }
                    state.page_clipboard = Some(PageClipboard {
                        triggers,
                        plocks,
                        fusions,
                    });
                    ui.close_menu();
                }
                if let Some(ref data) = state.page_clipboard {
                    if ui.button("Paste Page").clicked() {
                        let base = page * 16;
                        for i in 0..16 {
                            pattern.set_step_mask(base + i, data.triggers[i]);
                        }
                        for entry in &data.plocks {
                            let step = base + entry.step;
                            plock.masks.set_active(entry.instrument, step, true);
                            plock
                                .field_masks
                                .set_raw(entry.instrument, step, entry.field_mask);
                            for (field, &value) in entry.values.iter().enumerate() {
                                plock.values.set(entry.instrument, step, field, value);
                            }
                        }
                        replace_page_fusions_for_ui(pattern, params, plock, page, &data.fusions);
                        ui.close_menu();
                    }
                }
                if ui.button("Clear Page").clicked() {
                    let base = page * 16;
                    for i in 0..16 {
                        pattern.set_step_mask(base + i, 0);
                        for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
                            plock.clear(inst, base + i);
                        }
                    }
                    clear_page_fusions_for_ui(pattern, page);
                    ui.close_menu();
                }
            });
        }
        ui.add_space(16.0);
        let follow_btn = egui::Button::new(if state.follow_mode {
            "Follow ON"
        } else {
            "Follow OFF"
        })
        .min_size(Vec2::new(80.0, 22.0))
        .fill(if state.follow_mode {
            Color32::from_rgb(50, 150, 50)
        } else {
            Color32::from_rgb(80, 80, 80)
        });
        if ui.add(follow_btn).clicked() {
            state.follow_mode = !state.follow_mode;
        }
        ui.add_space(16.0);
        ui.label(RichText::new("Len:").strong());
        ui.add(widgets::ParamSlider::for_param(&params.pattern_length, setter).with_width(60.0));
        for &len in &[16, 32, 48, 64] {
            let is_active = params.pattern_length.value() == len;
            let btn = egui::Button::new(format!("{}", len))
                .min_size(Vec2::new(32.0, 22.0))
                .fill(if is_active {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(40, 40, 40)
                });
            if ui.add(btn).clicked() {
                setter.set_parameter(&params.pattern_length, len);
            }
        }
        ui.add_space(8.0);
        let current_len = params.pattern_length.value() as usize;
        let can_double = current_len <= 32;
        let x2_btn = egui::Button::new("x2").min_size(Vec2::new(32.0, 22.0));
        let response = ui.add_enabled(can_double, x2_btn);
        if response.clicked() {
            for i in 0..current_len {
                pattern.set_step_mask(current_len + i, pattern.load_step_mask(i));
                for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
                    if plock.masks.is_active(inst, i) {
                        let field_mask = plock.field_masks.get_raw(inst, i);
                        plock.masks.set_active(inst, current_len + i, true);
                        plock.field_masks.set_raw(inst, current_len + i, field_mask);
                        for field in 0..crate::plock::FIELD_COUNT {
                            let value = plock.values.get(inst, i, field);
                            plock.values.set(inst, current_len + i, field, value);
                        }
                    }
                }
            }
            duplicate_fusions_for_x2(pattern, params, plock, current_len);
            setter.set_parameter(&params.pattern_length, (current_len * 2) as i32);
        }
    });
    ui.add_space(4.0);

    // Follow mode: auto-switch page based on playhead
    if state.follow_mode {
        let master_step = current_step.load(Ordering::Relaxed) as usize;
        let target_page = master_step / 16;
        if target_page < 4 {
            state.current_page = target_page;
        }
    }

    let page_offset = state.current_page * 16;
    let fusion_mode_active = fusion_modifier_pressed(ui);
    if !fusion_mode_active {
        for selection_start in state.fusion_selection_start.iter_mut() {
            *selection_start = None;
        }
    }
    let mut fusion_inline_edit_rect = None;
    let mut fusion_editing_started_this_frame = false;

    egui::Grid::new("pattern-grid")
        .spacing(Vec2::new(4.0, 4.0))
        .show(ui, |ui| {
            // Header row --- use exact same widths as instrument rows below
            let header_item = |ui: &mut egui::Ui, text: &str, width: f32| {
                ui.add_sized(
                    Vec2::new(width, 20.0),
                    egui::Label::new(RichText::new(text).strong().size(10.0)),
                );
            };
            header_item(ui, "", 32.0);        // instrument label
            header_item(ui, "Vol", 40.0);     // volume slider
            header_item(ui, "M", 24.0);       // mute
            header_item(ui, "S", 24.0);       // solo
            header_item(ui, "T", 24.0);       // test
            // Steps container with tighter spacing (showing steps of current page)
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for local_step in 0..16 {
                    let global_step = page_offset + local_step;
                    let is_current = (current_step.load(Ordering::Relaxed) as usize) == global_step;
                    let text = if local_step % 4 == 0 {
                        RichText::new(format!("{}", global_step + 1)).strong().size(10.0)
                    } else {
                        RichText::new(format!("{}", global_step + 1)).size(10.0)
                    };
                    let label = if is_current {
                        RichText::new(text.text())
                            .strong()
                            .color(Color32::YELLOW)
                            .size(10.0)
                    } else {
                        text
                    };
                    ui.add_sized(
                        Vec2::new(20.0, 20.0),
                        egui::Label::new(label),
                    );
                }
            });
            header_item(ui, "Hum", 40.0);
            header_item(ui, "Push", 45.0);
            header_item(ui, "Len", 35.0);
            ui.end_row();

            let master_length = params.pattern_length.value() as usize;

            // Instrument rows
            for inst in 0..DrumVoice::COUNT {
                let row = &mixer_rows[inst];

                // Instrument label (clickable)
                let label_btn = egui::Button::new(
                    RichText::new(crate::instrument_registry::INSTRUMENTS[inst].label)
                        .monospace()
                        .size(11.0),
                )
                .min_size(Vec2::new(32.0, 22.0))
                .fill(if state.selected_instrument == inst {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(28, 28, 28)
                });
                if ui.add(label_btn).clicked() {
                    state.selected_instrument = inst;
                }

                // Volume par lane
                let inst_state = &sound_settings.instruments[inst];
                let mut lane_vol = f32::from_bits(inst_state.volume.load(Ordering::Relaxed));
                if draw_volume_db_slider(ui, &mut lane_vol, 40.0, false).changed() {
                    inst_state.volume.store(lane_vol.to_bits(), Ordering::Relaxed);
                    sound_settings.bump_version();
                }

                // Mute / Solo / Test
                draw_bool_toggle(ui, setter, row.mute, "M", "Mute");
                let solo_clicked = draw_bool_toggle(ui, setter, row.solo, "S", "Solo");
                if solo_clicked && params.auto_edit.value() {
                    state.selected_instrument = inst;
                }
                let test_btn = egui::Button::new("T").min_size(Vec2::new(24.0, 20.0));
                if ui.add(test_btn).on_hover_text("Test").clicked() {
                    voice_test_triggers[inst].store(true, Ordering::Release);
                }

                // Load fusions for this instrument. UI can allocate; audio sync uses fixed buffers.
                let fusions = pattern.load_fusions(inst);

                // 16 fixed columns of current page. A fusion is rendered as one
                // wide widget covering the same total width as its source cells.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let mut local_step = 0usize;
                    while local_step < 16 {
                        let global_step = page_offset + local_step;
                        let beyond_len = global_step >= master_length;
                        if beyond_len {
                            ui.allocate_space(Vec2::new(20.0, 20.0));
                            local_step += 1;
                            continue;
                        }

                        let fusion_info = fusion_containing(&fusions, global_step);
                        let (fusion_idx, fusion_group) = fusion_info
                            .map(|(idx, group)| (Some(idx), Some(group)))
                            .unwrap_or((None, None));
                        let source_step = fusion_group
                            .map(|group| group.start_cell as usize)
                            .unwrap_or(global_step);
                        let is_fusion = fusion_group.is_some();
                        let is_fusion_start = fusion_group
                            .map(|group| group.is_start(global_step))
                            .unwrap_or(false);

                        if is_fusion && !is_fusion_start {
                            // Internal cells are covered by the start cell's
                            // wide widget. Skipping them removes the visible
                            // subdivisions while preserving total row width.
                            local_step += 1;
                            continue;
                        }

                        let visible_span = fusion_group
                            .map(|group| {
                                let visible_end = (group.end_cell as usize)
                                    .min(page_offset + 15)
                                    .min(master_length.saturating_sub(1));
                                visible_end.saturating_sub(global_step) + 1
                            })
                            .unwrap_or(1);
                        let cell_width = 20.0 * visible_span as f32
                            + 6.0 * visible_span.saturating_sub(1) as f32;

                        let active = pattern.is_active(source_step, inst);
                        let current_track_step = current_steps[inst].load(Ordering::Relaxed) as usize;
                        let is_current = if let Some(group) = fusion_group {
                            group.contains(current_track_step)
                        } else {
                            current_track_step == global_step
                        };
                        let has_plock = plock.masks.is_active(inst, source_step);
                        let plock_mask = if has_plock {
                            plock.field_masks.get(inst, source_step)
                        } else {
                            0
                        };
                        let all_bits = (1u64 << crate::plock::FIELD_COUNT) - 1;
                        let is_snapshot = has_plock && plock_mask == all_bits;
                        let has_seq_plock = params.seq_plock_state.state.is_active(inst, source_step);
                        let is_fusion_selection_start = fusion_mode_active
                            && state.fusion_selection_start[inst] == Some(global_step);
                        let fusion_selection_blink_on = is_fusion_selection_start
                            && ((ui.input(|input| input.time) * 3.2) as u64 % 2 == 0);
                        if is_fusion_selection_start {
                            ui.ctx().request_repaint();
                        }

                        let base_bg = if state.sequencer_mode {
                            if active && has_seq_plock {
                                Color32::from_rgb(168, 85, 247)
                            } else if has_seq_plock {
                                Color32::from_rgb(126, 34, 206)
                            } else if active {
                                Color32::from_rgb(56, 132, 255)
                            } else if is_current {
                                Color32::from_rgb(48, 48, 48)
                            } else {
                                Color32::from_rgb(28, 28, 28)
                            }
                        } else if active && has_plock {
                            if is_snapshot {
                                Color32::from_rgb(220, 50, 50)
                            } else {
                                Color32::from_rgb(255, 140, 0)
                            }
                        } else if active {
                            Color32::from_rgb(56, 132, 255)
                        } else if has_plock {
                            if is_snapshot {
                                Color32::from_rgb(160, 30, 30)
                            } else {
                                Color32::from_rgb(180, 100, 0)
                            }
                        } else if is_current {
                            Color32::from_rgb(48, 48, 48)
                        } else if local_step < 4 || (8..12).contains(&local_step) {
                            Color32::from_rgb(32, 32, 32)
                        } else {
                            Color32::from_rgb(40, 40, 40)
                        };
                        let bg = if is_fusion_selection_start && fusion_selection_blink_on {
                            Color32::from_rgb(56, 132, 255)
                        } else {
                            base_bg
                        };

                        let text = if is_fusion_selection_start {
                            "X".to_string()
                        } else if let Some(group) = fusion_group {
                            if is_fusion_start {
                                group.step_count.to_string()
                            } else {
                                String::new()
                            }
                        } else if active {
                            "X".to_string()
                        } else {
                            ".".to_string()
                        };
                        let text = if is_fusion_selection_start {
                            RichText::new(text).size(10.0).strong()
                        } else {
                            RichText::new(text).size(10.0)
                        };
                        let stroke = if is_fusion_selection_start {
                            egui::Stroke::new(1.0, Color32::from_rgb(120, 200, 255))
                        } else {
                            egui::Stroke::NONE
                        };
                        let btn = egui::Button::new(text).fill(bg).stroke(stroke);
                        let is_editing_fusion = fusion_idx
                            .map(|idx| state.fusion_editing == Some((inst, idx)))
                            .unwrap_or(false);
                        let editing_any_fusion = state.fusion_editing.is_some();
                        let mut response = if is_editing_fusion {
                            if let (Some(idx), Some(group)) = (fusion_idx, fusion_group) {
                                let mut step_count = group.step_count as i32;
                                let edit_response = ui.add_sized(
                                    Vec2::new(cell_width, 20.0),
                                    egui::DragValue::new(&mut step_count)
                                        .speed(1.0)
                                        .range(1..=64),
                                );
                                fusion_inline_edit_rect = Some(edit_response.rect);

                                if edit_response.changed() {
                                    let mut new_fusions = fusions.clone();
                                    if let Some(group) = new_fusions.get_mut(idx) {
                                        group.step_count = step_count as u8;
                                        pattern.store_fusions(inst, &new_fusions);
                                    }
                                }

                                let finish_key = ui.input(|i| {
                                    i.key_pressed(egui::Key::Enter)
                                        || i.key_pressed(egui::Key::Escape)
                                });

                                if finish_key {
                                    finish_fusion_editing_for_ui(pattern, state);
                                }

                                edit_response
                            } else {
                                ui.add_sized(Vec2::new(cell_width, 20.0), btn)
                            }
                        } else {
                            ui.add_sized(Vec2::new(cell_width, 20.0), btn)
                        };
                        if let Some(group) = fusion_group {
                            response = response.on_hover_text(format!(
                                "Fusion {}-{}: {} pulses over {} cells\nClick: toggle fusion\nDouble-click: edit pulses in this cell",
                                group.start_cell as usize + 1,
                                group.end_cell as usize + 1,
                                group.step_count,
                                group.cell_span()
                            ));
                        }

                        if !editing_any_fusion && response.double_clicked() && fusion_idx.is_some() {
                            if let Some(idx) = fusion_idx {
                                state.fusion_editing = Some((inst, idx));
                                fusion_editing_started_this_frame = true;
                            }
                        } else if !editing_any_fusion
                            && response.clicked()
                            && fusion_mode_active
                        {
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
                        } else if !editing_any_fusion && response.clicked() {
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

                        if !editing_any_fusion {
                            response.context_menu(|ui| {
                                if let Some(idx) = fusion_idx {
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
                                            if state.fusion_editing == Some((inst, idx)) {
                                                state.fusion_editing = None;
                                            }
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
                                        is_fusion,
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

                        local_step += visible_span.max(1);
                    }
                });

                // Hum / Push / Len (compact sliders avec valeurs formatées stables)
                ui.horizontal(|ui| {
                    ui.add(widgets::ParamSlider::for_param(hums[inst], setter).with_width(32.0).without_value());
                    let hum_pct = (hums[inst].value() * 100.0) as i32;
                    ui.label(RichText::new(format!("{:>3}%", hum_pct)).monospace().size(9.0)).on_hover_text("Humanize");
                });
                ui.horizontal(|ui| {
                    ui.add(widgets::ParamSlider::for_param(pushes[inst], setter).with_width(32.0).without_value());
                    let push_val = pushes[inst].value() as i32;
                    ui.label(RichText::new(format!("{:>+3} ms", push_val)).monospace().size(9.0)).on_hover_text("Push/Pull");
                });
                draw_track_length_control(ui, setter, params, lengths[inst], inst, master_length);

                ui.end_row();
            }
        });

    // Mode switch under the sequencer grid
    let mut fusion_edit_box_rect = None;
    ui.horizontal(|ui| {
        ui.add_space(32.0); // indent to align with grid
        ui.label(RichText::new("Plock mode:").strong().size(11.0));
        let seq_btn = egui::Button::new(if state.sequencer_mode {
            "Sequencer"
        } else {
            "Sound"
        })
        .min_size(Vec2::new(70.0, 22.0))
        .fill(if state.sequencer_mode {
            Color32::from_rgb(147, 51, 234) // violet
        } else {
            Color32::from_rgb(234, 120, 50) // orange
        });
        if ui.add(seq_btn).clicked() {
            state.sequencer_mode = !state.sequencer_mode;
        }
        ui.label(
            RichText::new("Right-click steps for plocks")
                .small()
                .color(Color32::from_rgb(120, 120, 120)),
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        fusion_edit_box_rect = Some(draw_fusion_edit_box(ui, pattern, state, fusion_mode_active));
    });

    if !fusion_editing_started_this_frame {
        close_fusion_editing_on_outside_click(
            ui,
            pattern,
            state,
            fusion_inline_edit_rect,
            fusion_edit_box_rect,
        );
    }
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
    ui.label(RichText::new("Sound Editor").strong());
    state.selected_instrument = state.selected_instrument.min(DrumVoice::COUNT - 1);

    // Instrument tabs
    ui.horizontal(|ui| {
        for (i, label) in crate::instrument_registry::INSTRUMENTS
            .iter()
            .map(|d| d.label)
            .enumerate()
        {
            let selected = state.selected_instrument == i;
            let btn = egui::Button::new(RichText::new(label).monospace().size(11.0))
                .min_size(Vec2::new(32.0, 22.0))
                .fill(if selected {
                    Color32::from_rgb(56, 132, 255)
                } else {
                    Color32::from_rgb(36, 36, 36)
                });
            if ui.add(btn).clicked() {
                state.selected_instrument = i;
            }
        }
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
            ui.set_width(ui.available_width());

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

            // ------ Volume global de l'instrument ------
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Volume").strong().size(14.0));
                    ui.add_space(8.0);
                    if draw_volume_db_slider(ui, &mut vol, 180.0, true).changed() {
                        store_field(inst, crate::instrument_registry::StandardField::Volume, vol);
                        changed = true;
                    }
                });
            });
            ui.add(egui::Separator::default().spacing(12.0));

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
                ui.group(|ui| {
            ui.label(RichText::new(family.label()).strong().size(13.0));
            ui.separator();

            ui.horizontal(|ui| {
                // Left column: params
                ui.vertical(|ui| {
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
                                ui.label(label_text);

                                // Frequency display mode switch for Kick and B8
                                let is_bass_drum = state.selected_instrument == 0 || state.selected_instrument == 11;
                                if def.field == crate::instrument_registry::StandardField::Freq && is_bass_drum {
                                    let freq_mode_param = if state.selected_instrument == 0 {
                                        &params.freq_mode_kick
                                    } else {
                                        &params.freq_mode_bassdrum808
                                    };
                                    let mut freq_in_notes = freq_mode_param.value();
                                    if ui.checkbox(&mut freq_in_notes, "Notes").changed() {
                                        setter.set_parameter(freq_mode_param, freq_in_notes);
                                        // Snap frequency to exact note when switching to Notes mode
                                        if freq_in_notes {
                                            let ratio = instrument.freq_display_ratio;
                                            let snapped_note = freq_to_note(freq * ratio).round();
                                            freq = note_to_freq(snapped_note) / ratio;
                                            store_field(inst, crate::instrument_registry::StandardField::Freq, freq);
                                            changed = true;
                                        }
                                    }
                                }

                                // Check if we're in note display mode for bass drums
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
                                    // Special case: frequency in note mode for bass drums
                                    if freq_in_notes && field == crate::instrument_registry::StandardField::Freq {
                                        let ratio = instrument.freq_display_ratio;
                                        let note_val = freq_to_note(freq * ratio).round();
                                        ui.label(RichText::new(note_name(note_val)).monospace().size(14.0));
                                        if ui.button("-").clicked() {
                                            let new_note = (note_val - 1.0).max(0.0);
                                            freq = note_to_freq(new_note) / ratio;
                                            store_field(inst, field, freq);
                                            changed = true;
                                        }
                                        if ui.button("+").clicked() {
                                            let new_note = (note_val + 1.0).min(127.0);
                                            freq = note_to_freq(new_note) / ratio;
                                            store_field(inst, field, freq);
                                            changed = true;
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
                                        let slider = LocalParamSlider::new(value, *min..=*max)
                                            .logarithmic(*logarithmic)
                                            .with_width(120.0);
                                        if let Some(s) = suffix {
                                            // Note: LocalParamSlider doesn't support suffix in the same way
                                            // We'll add it to the label or handle it separately if needed
                                        }
                                        if ui.add(slider).changed() {
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
                                    let mut checked = *value >= 0.5;
                                    if ui.checkbox(&mut checked, "").changed() {
                                        *value = if checked { 1.0 } else { 0.0 };
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
                                ui.label(def.label);
                                // Boolean toggle for on/off switches (min=0, max=1)
                                if def.min == 0.0 && def.max == 1.0 && def.label.to_lowercase().contains("pre-filter") {
                                    let mut checked = param.value() >= 0.5;
                                    if ui.checkbox(&mut checked, "").changed() {
                                        setter.set_parameter(param, if checked { 1.0 } else { 0.0 });
                                    }
                                // Saturation Type: show combobox with names instead of number slider
                                } else if def.label.to_lowercase().contains("saturation type") {
                                    let current_val = param.value() as i32;
                                    let type_names = ["None", "SoftClip", "Valve", "Transistor", "HardClip", "Tape"];
                                    let current_name = type_names.get(current_val as usize).unwrap_or(&"None");
                                    egui::ComboBox::from_id_salt(def.name)
                                        .width(100.0)
                                        .selected_text(*current_name)
                                        .show_ui(ui, |ui| {
                                            for (idx, name) in type_names.iter().enumerate() {
                                                if ui.selectable_label(idx as i32 == current_val, *name).clicked() {
                                                    setter.set_parameter(param, idx as f32);
                                                }
                                            }
                                        });
                                // Cymbal Noise Type: show combobox with names
                                } else if def.label.to_lowercase().contains("noise type") {
                                    let current_val = param.value() as i32;
                                    let type_names = ["White", "Pink", "Brown", "Blue"];
                                    let current_name = type_names.get(current_val as usize).unwrap_or(&"White");
                                    egui::ComboBox::from_id_salt(def.name)
                                        .width(100.0)
                                        .selected_text(*current_name)
                                        .show_ui(ui, |ui| {
                                            for (idx, name) in type_names.iter().enumerate() {
                                                if ui.selectable_label(idx as i32 == current_val, *name).clicked() {
                                                    setter.set_parameter(param, idx as f32);
                                                }
                                            }
                                        });
                                // Kick Click Type: show combobox with names
                                } else if def.label.to_lowercase().contains("click type") {
                                    let current_val = param.value() as i32;
                                    let type_names = ["Soft", "Medium", "Hard"];
                                    let current_name = type_names.get(current_val as usize).unwrap_or(&"Medium");
                                    egui::ComboBox::from_id_salt(def.name)
                                        .width(100.0)
                                        .selected_text(*current_name)
                                        .show_ui(ui, |ui| {
                                            for (idx, name) in type_names.iter().enumerate() {
                                                if ui.selectable_label(idx as i32 == current_val, *name).clicked() {
                                                    setter.set_parameter(param, idx as f32);
                                                }
                                            }
                                        });
                                } else {
                                    ui.add(widgets::ParamSlider::for_param(param, setter).with_width(120.0));
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
                                    ui.label("Algorithm");
                                    let algo_names: Vec<&str> = algos.iter().map(|a| a.name).collect();
                                    algo_combo(ui, setter, algo_param, &algo_names);
                                });
                            }
                        }
                    }

                    // Mix checkbox inside OUTPUT family
                    if family == crate::instrument_registry::ParamFamily::Output {
                        let mix_param = params.mixes()[state.selected_instrument];
                        ui.horizontal(|ui| {
                            ui.label("Mix");
                            let mut mix = mix_param.value();
                            if ui.add(egui::Checkbox::new(&mut mix, "")).changed() {
                                setter.set_parameter(mix_param.into(), mix);
                            }
                        });
                    }

                    // Legend inside ENV family (under the params)
                    if family == crate::instrument_registry::ParamFamily::Env {
                        let has_attack = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::Attack);
                        let has_hold = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::Hold);
                        let has_release = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::Release);
                        ui.horizontal(|ui| {
                            let legend = |ui: &mut egui::Ui, color: Color32, text: &str| {
                                ui.label(RichText::new("- ").color(color));
                                ui.label(text);
                            };
                            if has_attack {
                                legend(ui, Color32::from_rgb(255, 220, 80), "A");
                            }
                            if has_hold {
                                legend(ui, Color32::from_rgb(140, 220, 255), "H");
                            }
                            legend(ui, Color32::from_rgb(100, 180, 255), "D");
                            if has_release {
                                legend(ui, Color32::from_rgb(180, 120, 255), "R");
                            }
                        });
                    }
                });

                // Right column: graphs
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
                });
            }
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
                .color(Color32::from_rgb(56, 132, 255)),
        );
    } else {
        ui.label(
            RichText::new("Maj for fusion mode")
                .size(11.0)
                .color(Color32::from_rgb(120, 120, 120)),
        );
    }
}

fn draw_fusion_edit_box(
    ui: &mut egui::Ui,
    pattern_for_ui: &SharedPattern,
    state: &mut EditorUIState,
    fusion_mode_active: bool,
) -> egui::Rect {
    let box_size = Vec2::new(520.0, 28.0);

    ui.allocate_ui_with_layout(
        box_size,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(24, 24, 30))
                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(58, 58, 72)))
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

fn bool_checkbox(ui: &mut egui::Ui, setter: &ParamSetter, param: &BoolParam, label: &str) {
    let mut value = param.value();
    if ui.checkbox(&mut value, label).changed() {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
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
    egui::ComboBox::from_label(label)
        .selected_text(variants[current_idx])
        .show_ui(ui, |ui| {
            for (i, name) in variants.iter().enumerate() {
                let selected = i == current_idx;
                if ui.selectable_label(selected, *name).clicked() && !selected {
                    setter.begin_set_parameter(param);
                    setter.set_parameter(param, E::from_index(i));
                    setter.end_set_parameter(param);
                }
            }
        });
}

fn algo_combo(ui: &mut egui::Ui, setter: &ParamSetter, param: &IntParam, algo_names: &[&str]) {
    let current = param.value() as usize;
    let current_clamped = current.min(algo_names.len().saturating_sub(1));
    egui::ComboBox::from_label("")
        .selected_text(*algo_names.get(current_clamped).unwrap_or(&"?"))
        .show_ui(ui, |ui| {
            for (i, name) in algo_names.iter().enumerate() {
                let selected = i == current_clamped;
                if ui.selectable_label(selected, *name).clicked() && !selected {
                    setter.begin_set_parameter(param);
                    setter.set_parameter(param, i as i32);
                    setter.end_set_parameter(param);
                }
            }
        });
}

fn draw_bool_toggle(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &BoolParam,
    short_label: &str,
    hover_label: &str,
) -> bool {
    let enabled = param.value();
    let button = egui::Button::new(short_label)
        .min_size(Vec2::new(24.0, 20.0))
        .fill(if enabled {
            Color32::from_rgb(56, 132, 255)
        } else {
            Color32::from_rgb(36, 36, 36)
        });

    let response = ui.add(button).on_hover_text(hover_label);
    let clicked = response.clicked();
    if clicked {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, !enabled);
        setter.end_set_parameter(param);
    }
    clicked
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
                .color(Color32::from_rgb(130, 130, 145)),
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
