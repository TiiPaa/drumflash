//! Sound Editor: tabbed Sound/Track panel, editor rows, lane layout presets.

use crate::sequencer::SharedPattern;
use crate::sound_settings::SoundSettingsState;
use crate::synthesis::{self, DrumVoice, VoiceSettings};
use crate::track::{TrackInstrumentKind, TrackLayoutState};
use crate::ui::controls::{algo_combo, draw_track_length_control};
use crate::ui::editor_state::{
    schema_voice_idx, select_legacy_track, EditorUIState, LanePresetAction, SoundEditorTab,
};
use crate::ui::envelope_viz::{
    draw_amp_envelope, draw_buzz_filter_envelope, draw_buzz_gate_graph, draw_filter_envelope,
    draw_sample_amp_graph, draw_sample_filter_graph,
};
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

/// Left-aligned label that occupies EXACTLY the fixed column width, so every
/// row's control starts at the same x. `allocate_ui_with_layout` shrinks to the
/// text (ragged left edges) — we allocate the exact box and paint the label.
pub fn editor_label(ui: &mut egui::Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(EDITOR_LABEL_W, 22.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        f_sans_med(11.5),
        INK2(),
    );
}

/// Section header (separator + muted title) shared by the Sound and Track tabs.
pub fn editor_section_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);
    ui.label(RichText::new(title).font(f_sans_sb(10.5)).color(INK3()));
    ui.add_space(6.0);
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

/// Same track with a quantisation step (e.g. 1.0 for integer semitones).
fn draw_editor_slider_track_stepped(
    ui: &mut egui::Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    logarithmic: bool,
    track_w: f32,
    step: f32,
) -> egui::Response {
    slider::draw_track(
        ui,
        value,
        min,
        max,
        default,
        logarithmic,
        track_w.max(60.0),
        slider::TrackStyle::editor().with_step(step),
    )
}

fn draw_note_freq_mode_toggle(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    notes_active: bool,
) -> Option<bool> {
    // Same keycap-based renderer as every other segmented switch.
    let selected = if notes_active { 1 } else { 0 };
    let new = crate::ui::skeuo::segmented(ui, id_salt, &["Hz", "Note"], selected);
    if new != selected {
        Some(new == 1)
    } else {
        None
    }
}

fn draw_note_step_button(ui: &mut egui::Ui, left: bool) -> egui::Response {
    // Keycap with a PAINTED triangle (◂ / ▸) — the glyphs are missing from Plex.
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(24.0, 22.0), egui::Sense::click());
    crate::ui::skeuo::keycap(ui, rect, crate::ui::widgets::KeycapState::Rest);
    if resp.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, 5.0, Color32::from_black_alpha(60));
    }
    let c = rect.center();
    let s = 4.0;
    let tri = if left {
        vec![
            egui::pos2(c.x + s * 0.4, c.y - s),
            egui::pos2(c.x + s * 0.4, c.y + s),
            egui::pos2(c.x - s * 0.6, c.y),
        ]
    } else {
        vec![
            egui::pos2(c.x - s * 0.4, c.y - s),
            egui::pos2(c.x - s * 0.4, c.y + s),
            egui::pos2(c.x + s * 0.6, c.y),
        ]
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(tri, INK(), egui::Stroke::NONE));
    resp
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

        // Mode switch FIRST — a fixed position right after the label, so it does
        // NOT jump when toggling Hz <-> Note (and matches the mockup: switch, then
        // slider). The slider/stepper fills the rest; the value stays right-aligned.
        let mode_change = draw_note_freq_mode_toggle(ui, id_salt, notes_active);

        let mut value_changed = false;
        let mut row_response = ui.allocate_response(Vec2::ZERO, egui::Sense::hover());

        if notes_active {
            let note_val = freq_to_note(*value * ratio).round();
            if draw_note_step_button(ui, true).clicked() {
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
            if draw_note_step_button(ui, false).clicked() {
                let new_note = (note_val + 1.0).min(127.0);
                *value = note_to_freq(new_note) / ratio;
                value_changed = true;
            }
        } else {
            let track_w = (ui.available_width() - EDITOR_VALUE_W - 8.0).max(60.0);
            row_response =
                draw_editor_slider_track(ui, value, min, max, default, logarithmic, track_w);
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
    draw_editor_slider_row_full(ui, label, value, min, max, default, logarithmic, suffix, 0.0)
}

/// Full editor slider row with an optional quantisation step (0 = continuous).
pub fn draw_editor_slider_row_full(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    logarithmic: bool,
    suffix: Option<&str>,
    step: f32,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, label);

        // Track flexes to fill the fixed-width params column.
        let track_w = (ui.available_width() - EDITOR_VALUE_W - 8.0).max(60.0);
        let response = if step > 0.0 {
            draw_editor_slider_track_stepped(ui, value, min, max, default, logarithmic, track_w, step)
        } else {
            draw_editor_slider_track(ui, value, min, max, default, logarithmic, track_w)
        };

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

/// Skeuo dropdown flush to the params column's right edge (matches the mockup:
/// dropdowns line up with slider values and toggles, not left after the label).
/// The caller draws `editor_label` first, then calls this in the same row.
fn right_aligned_select(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    selected: usize,
    options: &[&str],
) -> Option<usize> {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        styled_select(ui, id_salt, selected, options, 146.0).1
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

    let mut new_state = layout_state.clone();
    let mut changed = false;

    // Keep the name buffer in sync with the selected slot.
    if state.track_name_input_slot != Some(slot_idx) {
        state.track_name_input = slot.name.clone();
        state.track_name_input_slot = Some(slot_idx);
    }

    ui.add_space(2.0);

    // ---- Instrument ----
    editor_section_header(ui, "Instrument");

    // Name — same 146×26 keycap box as the Type/Aux Out dropdowns so the right
    // edges line up.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, "Name");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(146.0, CTL_HEIGHT), egui::Sense::hover());
            crate::ui::widgets::keycap_tex(ui, rect, crate::ui::widgets::KeycapState::Rest);
            let response = ui
                .allocate_new_ui(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink2(Vec2::new(8.0, 0.0)))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.track_name_input)
                                .id(egui::Id::new(("track_name", slot_idx)))
                                .char_limit(6)
                                .desired_width(f32::INFINITY)
                                .font(f_sans_med(12.0))
                                .text_color(Color32::WHITE)
                                .frame(false),
                        )
                    },
                )
                .inner;
            if state.track_name_focus_request {
                state.track_name_focus_request = false;
                response.request_focus();
            }
            if response.changed() {
                new_state.slots[slot_idx].name = state.track_name_input.clone();
                changed = true;
            }
        });
    });

    // Type (instrument kind)
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
        TrackInstrumentKind::Bd6smp,
        TrackInstrumentKind::Sd6smp,
        TrackInstrumentKind::Ch6smp,
        TrackInstrumentKind::Buzz,
    ];
    let current_kind = slot.kind;
    let kind_labels: Vec<&str> = kinds.iter().map(|k| k.default_name()).collect();
    let cur_kind_idx = kinds.iter().position(|k| *k == current_kind).unwrap_or(0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, "Type");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let (_, Some(i)) = crate::ui::widgets::styled_select(
                ui,
                "track_kind",
                cur_kind_idx,
                &kind_labels,
                146.0,
            ) {
                let kind = kinds[i];
                if kind != current_kind {
                    let label = kind.default_name();
                    new_state.slots[slot_idx].kind = kind;
                    new_state.slots[slot_idx].name = label.to_string();
                    new_state.slots[slot_idx].midi_note = kind.default_midi_note();
                    state.track_name_input = label.to_string();
                    changed = true;
                    // New kind → new voice: seed the slot's settings from the new
                    // instrument's defaults (the audio thread reinitializes it).
                    sound_settings.reset_slot_to_defaults(
                        slot_idx,
                        kind,
                        state.global_config.default_analog,
                    );
                    state.selected_instrument = slot_idx;
                }
            }
        });
    });

    // ---- Routing ----
    editor_section_header(ui, "Routing");

    // Main Mix (toggle)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, "Main Mix");
        let avail = ui.available_width();
        ui.add_space((avail - 34.0).max(0.0));
        let checked = slot.routing.main_on;
        if ui.add(ToggleSwitch::new(checked)).clicked() {
            new_state.slots[slot_idx].routing.main_on = !checked;
            changed = true;
        }
    });

    // Aux Out (dropdown)
    let current_out = slot.routing.out_select.index();
    let out_labels: Vec<String> = (0..=crate::track::MAX_TRACKS)
        .map(|i| {
            if i == 0 {
                "No Aux".to_string()
            } else {
                format!("Out {}", i)
            }
        })
        .collect();
    let out_refs: Vec<&str> = out_labels.iter().map(|s| s.as_str()).collect();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, "Aux Out");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let (_, Some(i)) = crate::ui::widgets::styled_select(
                ui,
                "track_out",
                current_out as usize,
                &out_refs,
                146.0,
            ) {
                if i as u8 != current_out {
                    new_state.assign_slot_output_exclusive(
                        slot_idx,
                        crate::track::TrackAudioOut::from_index(i as u8),
                    );
                    changed = true;
                }
            }
        });
    });

    // Choke group (dropdown) — when this slot triggers, every other active
    // slot in the same group is silenced (classic HH→OH, generalized).
    let current_choke = slot
        .routing
        .choke_group
        .min(crate::track::CHOKE_GROUP_COUNT) as usize;
    let choke_labels = ["None", "1", "2", "3", "4"];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, "Choke");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let (_, Some(i)) = crate::ui::widgets::styled_select(
                ui,
                "track_choke",
                current_choke,
                &choke_labels,
                146.0,
            ) {
                if i != current_choke {
                    new_state.slots[slot_idx].routing.choke_group = i as u8;
                    changed = true;
                }
            }
        });
    });

    // ---- MIDI ----
    editor_section_header(ui, "MIDI");

    // Channel (global, read-only)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, "Channel");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{}", layout_state.global_midi_channel))
                    .font(f_mono_med(12.0))
                    .color(Color32::WHITE),
            );
        });
    });

    // Note
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, "Note");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut note = slot.midi_note as i32;
            if ui
                .add(egui::DragValue::new(&mut note).range(0..=127).speed(1.0))
                .changed()
            {
                new_state.slots[slot_idx].midi_note = note.clamp(0, 127) as u8;
                changed = true;
            }
        });
    });

    // ---- Sequencing ----
    editor_section_header(ui, "Sequencing");
    let master_length = params.pattern_length.value().clamp(1, 64) as usize;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        editor_label(ui, "Length");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            draw_track_length_control(
                ui,
                setter,
                params,
                params.lengths()[slot_idx],
                slot_idx,
                master_length,
            );
        });
    });

    if changed {
        PersistentField::<TrackLayoutState>::set(&params.track_layout, new_state);
    }
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
            sound_settings.reset_slot_to_defaults(
                slot_idx,
                slot.kind,
                state.global_config.default_analog,
            );
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
    ui.painter()
        .rect_filled(header_rect, 0.0, PANEL_SKEUO_HEADER);
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
                    RichText::new("Lane Editor")
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

    // Mode toggle: two FLUSH tabs — full width, 50/50, no radius, hairline between,
    // blue when active, plaque colour (lightens on hover) when inactive. The only
    // segmented in the app without a keycap: it reads as two tabs at the edge.
    let tabs_rect = ui
        .allocate_exact_size(Vec2::new(ui.available_width(), 30.0), egui::Sense::hover())
        .0;
    let half = tabs_rect.width() * 0.5;
    for (idx, (tab, label, hover_txt)) in [
        (
            SoundEditorTab::Sound,
            "Sound",
            "Synthesis settings of the selected lane",
        ),
        (
            SoundEditorTab::Track,
            "Track",
            "Instrument type, MIDI note, routing, length",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let seg = egui::Rect::from_min_size(
            egui::pos2(tabs_rect.left() + idx as f32 * half, tabs_rect.top()),
            Vec2::new(half, tabs_rect.height()),
        );
        let selected = state.sound_editor_tab == tab;
        let resp = ui
            .interact(
                seg,
                ui.make_persistent_id(("lane_editor_tab", idx)),
                egui::Sense::click(),
            )
            .on_hover_text(hover_txt);
        let fill = if selected {
            BLUE()
        } else if resp.hovered() {
            PANEL_SKEUO_HOVER
        } else {
            PANEL_SKEUO_HEADER
        };
        ui.painter().rect_filled(seg, 0.0, fill);
        ui.painter().text(
            seg.center(),
            egui::Align2::CENTER_CENTER,
            label,
            f_mono_med(10.5),
            if selected { Color32::WHITE } else { INK2() },
        );
        if resp.clicked() {
            state.sound_editor_tab = tab;
        }
    }
    // Hairline between the two tabs + bottom border of the toggle band.
    ui.painter().vline(
        tabs_rect.center().x,
        tabs_rect.y_range(),
        egui::Stroke::new(1.0, LINE()),
    );
    ui.painter().hline(
        tabs_rect.x_range(),
        tabs_rect.bottom(),
        egui::Stroke::new(1.0, LINE()),
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

    // One-shot migration for sampler builds that persisted pitch in Hz.
    // The voice also understands the legacy marker, so audio is correct even
    // before the Sound tab is opened; opening it commits the semitone value.
    if matches!(voice_idx, 13 | 14 | 15) && inst.special_value(10) < 0.5 {
        let legacy_root = if voice_idx == 13 {
            60.0
        } else if voice_idx == 14 {
            200.0
        } else {
            8000.0
        };
        freq = if freq > 0.0 {
            (12.0 * (freq / legacy_root).log2()).clamp(-24.0, 24.0)
        } else {
            0.0
        };
        inst.frequency.store(freq.to_bits(), Ordering::Relaxed);
        inst.set_special(10, 1.0);
        // The old blob has no End parameter (reads 0) — restore full length.
        if inst.special_value(11) <= 0.0 {
            inst.set_special(11, 1.0);
        }
        changed = true;
    }

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
                    let standards = if matches!(voice_idx, 2 | 3 | 7 | 8 | 10 | 12 | 13 | 14 | 15)
                    {
                        // HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap, BD606, SD606 - use 0.0 as placeholder
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
                                    2 | 3 | 7 | 8 | 10 | 12 | 13 | 14 | 15 // HiHat, OpenHiHat, Ride, Cymbal, Perc1, Zap, BD606, SD606
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

            // Volume en tête (sans titre de section) — pleine largeur, comme les
            // sections sans graphe.
            let vol_changed = ui
                .scope(|ui| {
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
            // Graph sections keep a fixed params column so the ADSR/filter graph
            // keeps its space; graph-less sections fill the panel (wide sliders,
            // values/dropdowns/toggles flush right — like the mockup).
            let params_w = if has_graph {
                EDITOR_PARAMS_W
            } else {
                ui.available_width()
            };
            ui.horizontal(|ui| {
                // Left column: params (width-constrained so the graph keeps its space)
                ui.vertical(|ui| {
                    ui.set_max_width(params_w);
                    ui.set_width(params_w);
                    ui.spacing_mut().item_spacing.y = 9.0;
                    // Standard params for this family
                    // smp voices: One Shot bypasses the amp envelope — grey out
                    // the Env sliders (the One Shot switch is a special param,
                    // rendered below, and stays enabled).
                    let env_disabled = family == crate::instrument_registry::ParamFamily::Env
                        && matches!(voice_idx, 13 | 14 | 15)
                        && inst.special_value(2) > 0.5;
                    ui.add_enabled_ui(!env_disabled, |ui| {
                    for def in standard_defs.iter().filter(|d| {
                        d.family == family
                            && d.field != crate::instrument_registry::StandardField::Volume
                    }) {
                            ui.horizontal(|ui| {
                                let label_text = if def.field == crate::instrument_registry::StandardField::FilterFreq {
                                    let ft = crate::instrument_registry::filter_type_label(voice_idx);
                                    if ft.is_empty() {
                                        def.label.to_string()
                                    } else {
                                        format!("{} ({})", def.label, ft)
                                    }
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
                                        let default_value = if matches!(voice_idx, 13 | 14 | 15) {
                                            instrument.sound_settings_default[field as usize]
                                        } else {
                                            match field {
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
                                            }
                                        };
                                        // Relative-pitch voices: the Pitch slider
                                        // steps by 1 semitone (Pitch Fine covers
                                        // the cents).
                                        let smp_pitch = field
                                            == crate::instrument_registry::StandardField::Freq
                                            && matches!(voice_idx, 13 | 14 | 15);
                                        if draw_editor_slider_row_full(
                                            ui,
                                            &label_text,
                                            value,
                                            *min,
                                            *max,
                                            default_value,
                                            *logarithmic,
                                            *suffix,
                                            if smp_pitch { 1.0 } else { 0.0 },
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
                        // Multisample voices: Pitch Fine lives directly under
                        // the Pitch slider (it tunes the same parameter).
                        if def.field == crate::instrument_registry::StandardField::Freq
                            && matches!(voice_idx, 13 | 14 | 15)
                        {
                            let mut fine = inst.special_value(9);
                            if draw_editor_slider_row(
                                ui,
                                "Pitch Fine",
                                &mut fine,
                                -100.0,
                                100.0,
                                0.0,
                                false,
                                None,
                            )
                            .changed()
                            {
                                inst.set_special(9, fine);
                                sound_settings.bump_version();
                            }
                        }
                    }
                    });

                    // Special params for this family — stored PER SLOT so two
                    // slots of the same kind stay independent.
                    for def in special_defs.iter().filter(|d| d.family == family) {
                        // Pitch Fine is rendered directly under the Pitch slider.
                        if def.name.ends_with("_fine_tune") {
                            continue;
                        }
                        // Buzz gate controls render in their own sub-row, with
                        // the gate shape graph beside them (see below).
                        if voice_idx == 16 && def.name.starts_with("buzz_gate") {
                            continue;
                        }
                        // Multisample voices (*606): the Sample list only makes
                        // sense in fixed-sample mode — grey it out (don't hide
                        // it, to keep the layout stable) while Analog Mode
                        // (random multisample) is on.
                        let sample_disabled =
                            def.name.ends_with("_sample") && inst.special_value(0) > 0.5;
                        ui.add_enabled_ui(!sample_disabled, |ui| {
                        ui.horizontal(|ui| {
                            let current = inst.special_value(def.special_index);
                            let mut new_value = None;
                            // Multisample voices: Analog Mode / One Shot as switches
                            if def.name.ends_with("_analog_mode") || def.name.ends_with("_one_shot") {
                                let mut value = current;
                                if draw_editor_switch_row(ui, def.label, &mut value).changed() {
                                    new_value = Some(value);
                                }
                            // Multisample voices: Sample select 1..8 (stored 1-based)
                            } else if def.name.ends_with("_sample") {
                                let sample_names = ["1", "2", "3", "4", "5", "6", "7", "8"];
                                let current_idx = (current.round() as usize).clamp(1, 8) - 1;
                                editor_label(ui, def.label);
                                if let Some(idx) = right_aligned_select(ui, def.name, current_idx, &sample_names) {
                                    new_value = Some(idx as f32 + 1.0);
                                }
                            // Boolean toggle for on/off switches (min=0, max=1)
                            } else if def.min == 0.0 && def.max == 1.0 && def.label.to_lowercase().contains("pre-filter") {
                                let mut value = current;
                                if draw_editor_switch_row(ui, def.label, &mut value).changed() {
                                    new_value = Some(value);
                                }
                            // Saturation Type: show select with names instead of number slider
                            } else if def.label.to_lowercase().contains("saturation type") {
                                let type_names = ["None", "SoftClip", "Valve", "Transistor", "HardClip", "Tape"];
                                let current_idx = (current as usize).min(type_names.len().saturating_sub(1));
                                editor_label(ui, def.label);
                                if let Some(idx) = right_aligned_select(ui, def.name, current_idx, &type_names) {
                                    new_value = Some(idx as f32);
                                }
                            // Cymbal Noise Type: show select with names
                            } else if def.label.to_lowercase().contains("noise type") {
                                let type_names = ["White", "Pink", "Brown", "Blue"];
                                let current_idx = (current as usize).min(type_names.len().saturating_sub(1));
                                editor_label(ui, def.label);
                                if let Some(idx) = right_aligned_select(ui, def.name, current_idx, &type_names) {
                                    new_value = Some(idx as f32);
                                }
                            // Kick Click Type: show select with names
                            } else if def.label.to_lowercase().contains("click type") {
                                let type_names = ["Soft", "Medium", "Hard"];
                                let current_idx = (current as usize).min(type_names.len().saturating_sub(1));
                                editor_label(ui, def.label);
                                if let Some(idx) = right_aligned_select(ui, def.name, current_idx, &type_names) {
                                    new_value = Some(idx as f32);
                                }
                            // Buzz oscillator waveform: show select with names
                            } else if def.name.ends_with("_wave") {
                                let type_names = ["Sine", "Square", "Saw"];
                                let current_idx = (current as usize).min(type_names.len().saturating_sub(1));
                                editor_label(ui, def.label);
                                if let Some(idx) = right_aligned_select(ui, def.name, current_idx, &type_names) {
                                    new_value = Some(idx as f32);
                                }
                            // Buzz filter type: LP / HP / BP select
                            } else if def.name.ends_with("_filter_type") {
                                let type_names = ["LP", "HP", "BP"];
                                let current_idx = (current as usize).min(type_names.len().saturating_sub(1));
                                editor_label(ui, def.label);
                                if let Some(idx) = right_aligned_select(ui, def.name, current_idx, &type_names) {
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
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| algo_combo(ui, setter, algo_param, &algo_names),
                                    );
                                });
                            }
                        }
                    }

                });

                // Right column: graphs (gap so they aren't cramped against the params)
                if has_graph {
                    ui.add_space(16.0);
                }
                let sample_graph = if matches!(voice_idx, 13 | 14 | 15) {
                    let bank = if voice_idx == 13 {
                        crate::synthesis::sample_bank::bd606()
                    } else if voice_idx == 14 {
                        crate::synthesis::sample_bank::sd606()
                    } else {
                        crate::synthesis::sample_bank::ch606()
                    };
                    let hit_idx = (inst.special_value(1).round() as usize).clamp(1, 8) - 1;
                    // Legacy sessions (pitch marker unset) predate End: full length.
                    let end = if inst.special_value(10) < 0.5 {
                        1.0
                    } else {
                        inst.special_value(11)
                    };
                    Some((&bank.hits[hit_idx][..], inst.special_value(3), end))
                } else {
                    None
                };

                match family {
                    crate::instrument_registry::ParamFamily::Env => {
                        if let Some((hit, start, end)) = sample_graph {
                            draw_sample_amp_graph(
                                ui,
                                hit,
                                start,
                                end,
                                attack,
                                decay,
                                decay_curve,
                                inst.special_value(2) > 0.5,
                            );
                        } else {
                            // A-H-D bipolar: `release_curve` is repurposed as the
                            // attack curve; `decay_curve` shapes the decay.
                            draw_amp_envelope(ui, attack, release_curve, hold, decay, decay_curve);
                        }
                    }
                    crate::instrument_registry::ParamFamily::Filter => {
                        let has_filter_env = standard_defs.iter().any(|d| d.field == crate::instrument_registry::StandardField::FilterEnvAmount);
                        if has_filter_env {
                            let filter_curve = crate::synthesis::DrumVoice::from_index(voice_idx)
                                .and_then(|v| v.filter_env_curve())
                                .unwrap_or(decay_curve);
                            if let Some((hit, start, end)) = sample_graph {
                                draw_sample_filter_graph(
                                    ui,
                                    hit,
                                    start,
                                    end,
                                    filt,
                                    filter_env_amount,
                                    filter_env_decay,
                                    filter_curve,
                                );
                            } else if voice_idx == 16 {
                                // Buzz: A-H-D filter envelope (attack/hold/decay
                                // + bipolar curve) sweeping the cutoff.
                                draw_buzz_filter_envelope(
                                    ui,
                                    filt,
                                    filter_env_amount,
                                    inst.special_value(12), // Filter Attack
                                    inst.special_value(13), // Filter Hold
                                    filter_env_decay,
                                    inst.special_value(16), // Filter Atk Curve
                                    inst.special_value(15), // Filter Dec Curve
                                );
                            } else {
                                draw_filter_envelope(ui, filter_curve, filter_env_decay);
                            }
                        }
                    }
                    _ => {}
                }
            });

            // Buzz, Env family: the gate controls get their own row with the
            // gate shape graph BESIDE the sliders (not stacked under the amp
            // graph — stacking grew the section and shifted the layout).
            if family == crate::instrument_registry::ParamFamily::Env && voice_idx == 16 {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width(params_w);
                        ui.set_width(params_w);
                        ui.spacing_mut().item_spacing.y = 9.0;
                        for def in special_defs
                            .iter()
                            .filter(|d| d.family == family && d.name.starts_with("buzz_gate"))
                        {
                            let mut value = inst.special_value(def.special_index);
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
                                inst.set_special(def.special_index, value);
                                sound_settings.bump_version();
                            }
                        }
                    });
                    ui.add_space(16.0);
                    draw_buzz_gate_graph(
                        ui,
                        params.algos()[state.selected_instrument].value() == 1,
                        inst.special_value(0),
                        inst.special_value(1),
                        inst.special_value(2),
                    );
                });
            }
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
