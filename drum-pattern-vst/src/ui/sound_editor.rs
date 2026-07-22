//! Sound Editor: tabbed Sound/Track panel, editor rows, lane layout presets.

use crate::sound_settings::SoundSettingsState;
use crate::sequencer::SharedPattern;
use crate::synthesis::{self, DrumVoice, VoiceSettings};
use crate::track::{TrackInstrumentKind, TrackLayoutState};
use crate::ui::controls::{algo_combo, draw_track_length_control};
use crate::ui::editor_state::{
    select_legacy_track, schema_voice_idx, EditorUIState, LanePresetAction, SoundEditorTab,
};
use crate::ui::envelope_viz::{draw_amp_envelope, draw_filter_envelope};
use crate::ui::fmt::{freq_to_note, note_name, note_to_freq};
use crate::ui::pattern_bank::load_pattern_for_ui;
use crate::ui::slider;
use crate::ui::theme::*;
use crate::ui::widgets::{styled_select, ToggleSwitch};
use crate::{preset_dumps, DrumFlashParams};
use nih_plug::{params::persist::PersistentField, prelude::*};
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use std::sync::atomic::Ordering;

pub const EDITOR_LABEL_W: f32 = 138.0;
pub const EDITOR_PARAMS_W: f32 = 340.0;
pub const EDITOR_VALUE_W: f32 = 52.0;

pub fn format_editor_value(value: f32, suffix: Option<&str>) -> String {
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
pub fn editor_label(ui: &mut egui::Ui, text: &str) {
    ui.allocate_ui_with_layout(
        Vec2::new(EDITOR_LABEL_W, 22.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(RichText::new(text).font(f_sans_med(11.5)).color(INK2()));
        },
    );
}

pub fn draw_editor_slider_track(
    ui: &mut egui::Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    logarithmic: bool,
    track_w: f32,
) -> egui::Response {
    slider::draw_track(
        ui,
        value,
        min,
        max,
        default,
        logarithmic,
        track_w.max(60.0),
        slider::TrackStyle::editor(),
    )
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
    painter.rect_filled(rect, 6.0, PANEL2());
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, LINE2()),
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
            painter.rect_filled(seg.shrink(1.0), 5.0, BLUE());
        } else if response.hovered() {
            painter.rect_stroke(
                seg.shrink(1.0),
                5.0,
                egui::Stroke::new(1.0, BLUE()),
                egui::StrokeKind::Inside,
            );
        }
        if idx == 1 {
            painter.line_segment(
                [
                    egui::pos2(seg.left(), rect.top() + 3.0),
                    egui::pos2(seg.left(), rect.bottom() - 3.0),
                ],
                egui::Stroke::new(1.0, LINE2()),
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
                INK()
            } else {
                INK2()
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
        egui::Button::new(RichText::new(label).font(f_mono_sb(12.0)).color(INK2()))
            .min_size(Vec2::new(24.0, 22.0))
            .fill(PANEL2())
            .stroke(egui::Stroke::new(1.0, LINE2()))
            .corner_radius(5.0),
    )
}

pub struct EditorFrequencyRowResult {
    pub response: egui::Response,
    pub value_changed: bool,
    pub mode_change: Option<bool>,
}

fn draw_editor_frequency_row(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
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
                            .color(INK()),
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
            row_response = draw_editor_slider_track(ui, value, min, max, default, logarithmic, track_w);
            value_changed = row_response.changed();
            ui.allocate_ui_with_layout(
                Vec2::new(EDITOR_VALUE_W, 22.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.label(
                        RichText::new(format_editor_value(*value, Some("Hz")))
                            .font(f_mono(11.0))
                            .color(INK()),
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

pub fn draw_editor_slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    logarithmic: bool,
    suffix: Option<&str>,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, label);

        // Track flexes to fill the fixed-width params column.
        let track_w = (ui.available_width() - EDITOR_VALUE_W - 8.0).max(60.0);
        let response = draw_editor_slider_track(ui, value, min, max, default, logarithmic, track_w);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format_editor_value(*value, suffix))
                    .font(f_mono(11.0))
                    .color(INK()),
            );
        });
        response
    })
    .inner
}

pub fn draw_editor_switch_row(ui: &mut egui::Ui, label: &str, value: &mut f32) -> egui::Response {
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
                .color(INK3()),
        );
        return;
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new(format!("Slot {} — {}", slot_idx + 1, slot.name))
            .font(f_sans_sb(12.0))
            .color(Color32::WHITE),
    );

    let mut new_state = layout_state.clone();
    let mut changed = false;

    ui.add_space(12.0);
    ui.label(RichText::new("Name").font(f_sans_med(10.5)).color(INK3()));
    if state.track_name_input_slot != Some(slot_idx) {
        state.track_name_input = slot.name.clone();
        state.track_name_input_slot = Some(slot_idx);
    }
    ui.horizontal(|ui| {
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.track_name_input)
                .id(egui::Id::new(("track_name", slot_idx)))
                .char_limit(6)
                .desired_width(180.0)
                .font(f_sans_med(12.0))
                .text_color(Color32::WHITE),
        );
        if state.track_name_focus_request {
            state.track_name_focus_request = false;
            response.request_focus();
        }
        if response.changed() {
            new_state.slots[slot_idx].name = state.track_name_input.clone();
            changed = true;
        }
    });

    ui.add_space(12.0);
    ui.label(
        RichText::new("Instrument")
            .font(f_sans_med(10.5))
            .color(INK3()),
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
                    new_state.slots[slot_idx].kind = kind;
                    new_state.slots[slot_idx].name = label.to_string();
                    new_state.slots[slot_idx].midi_note = kind.default_midi_note();
                    state.track_name_input = label.to_string();
                    changed = true;
                    // New kind → new voice: align the slot's settings with the
                    // new instrument's defaults (the audio thread reinitializes
                    // the voice via last_slot_kinds detection).
                    sound_settings.reset_slot_to_defaults(slot_idx, kind, state.global_config.default_analog);
                    // Keep the selection on this slot; the Sound tab schema
                    // follows the slot's kind automatically.
                    state.selected_instrument = slot_idx;
                }
            }
        });

    ui.add_space(16.0);
    ui.label(RichText::new("Routing").font(f_sans_med(10.5)).color(INK3()));
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
        ui.label(RichText::new("Out").font(f_sans_med(11.0)).color(INK3()));
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
    ui.label(RichText::new("MIDI").font(f_sans_med(10.5)).color(INK3()));
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(70.0, 20.0),
            egui::Label::new(
                RichText::new("Channel").font(f_sans_med(11.0)).color(INK2()),
            ),
        );
        let current_channel = layout_state.global_midi_channel;
        ui.label(
            RichText::new(format!("{}", current_channel))
                .font(f_mono_med(12.0))
                .color(Color32::WHITE),
        );
        ui.add_space(8.0);
        ui.add_sized(
            Vec2::new(40.0, 20.0),
            egui::Label::new(
                RichText::new("Note").font(f_sans_med(11.0)).color(INK2()),
            ),
        );
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
            .color(INK3()),
    );
    let master_length = params.pattern_length.value().clamp(1, 64) as usize;
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(70.0, 20.0),
            egui::Label::new(RichText::new("Length").font(f_sans_med(11.0)).color(INK2())),
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

pub fn apply_lane_layout_preset(
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
        crate::ui::grid::clear_all_fusions(pattern);
        state.last_loaded_slot = None;
    }

    for (slot_idx, slot) in layout.slots.iter().enumerate() {
        if slot.active {
            sound_settings.reset_slot_to_defaults(slot_idx, slot.kind, state.global_config.default_analog);
        }
    }

    let selected_slot = layout.active_slot_indices().next().unwrap_or(0);
    PersistentField::<TrackLayoutState>::set(&params.track_layout, layout);
    state.add_module_popup = None;
    select_legacy_track(state, selected_slot);
}

pub fn apply_lane_preset_action(
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
pub fn draw_sound_panel(
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
    ui.painter().rect_filled(header_rect, 0.0, PANEL());
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        egui::Stroke::new(1.0, LINE()),
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
                ui.label(RichText::new(header_name).font(f_mono(11.0)).color(INK3()));
                // (Engine selector belongs to the future modular phase — omitted for now.)
            });
        },
    );

    let tabs_rect = ui
        .allocate_exact_size(Vec2::new(ui.available_width(), 45.0), egui::Sense::hover())
        .0;
    ui.painter().rect_filled(tabs_rect, 0.0, PANEL());
    ui.painter().hline(
        tabs_rect.x_range(),
        tabs_rect.bottom(),
        egui::Stroke::new(1.0, LINE()),
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
                            .color(if selected { Color32::WHITE } else { INK2() }),
                    )
                    .min_size(Vec2::new(tab_w, CTL_HEIGHT))
                    .fill(if selected { BLUE() } else { PANEL2() })
                    .stroke(egui::Stroke::new(1.0, if selected { BLUE() } else { LINE2() }))
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
                    draw_editor_slider_row(
                        ui,
                        "Volume",
                        &mut vol,
                        0.0,
                        2.0,
                        VoiceSettings::default().volume,
                        false,
                        Some(""),
                    )
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
                if !fam_has_std && !fam_has_special {
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
                ui.label(RichText::new(section_title).font(f_sans_sb(10.5)).color(INK3()));
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
                                            VoiceSettings::default().frequency,
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
                                        let default_value = match field {
                                            crate::instrument_registry::StandardField::Freq => VoiceSettings::default().frequency,
                                            crate::instrument_registry::StandardField::Decay => VoiceSettings::default().decay,
                                            crate::instrument_registry::StandardField::Volume => VoiceSettings::default().volume,
                                            crate::instrument_registry::StandardField::FilterFreq => VoiceSettings::default().filter_freq,
                                            crate::instrument_registry::StandardField::Attack => VoiceSettings::default().attack,
                                            crate::instrument_registry::StandardField::Release => VoiceSettings::default().release,
                                            crate::instrument_registry::StandardField::DecayCurve => VoiceSettings::default().decay_curve,
                                            crate::instrument_registry::StandardField::ReleaseCurve => VoiceSettings::default().release_curve,
                                            crate::instrument_registry::StandardField::Hold => VoiceSettings::default().hold,
                                            crate::instrument_registry::StandardField::FilterEnvAmount => VoiceSettings::default().filter_env_amount,
                                            crate::instrument_registry::StandardField::FilterEnvDecay => VoiceSettings::default().filter_env_decay,
                                            crate::instrument_registry::StandardField::Analog => VoiceSettings::default().analog,
                                            crate::instrument_registry::StandardField::Stereo => VoiceSettings::default().stereo,
                                        };
                                        if draw_editor_slider_row(
                                            ui,
                                            &label_text,
                                            value,
                                            *min,
                                            *max,
                                            default_value,
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
                                    def.default,
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
                            let filter_curve = crate::synthesis::DrumVoice::from_index(voice_idx)
                                .and_then(|v| v.filter_env_curve())
                                .unwrap_or(decay_curve);
                            draw_filter_envelope(ui, filter_curve, filter_env_decay);
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

pub fn store_field(
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
