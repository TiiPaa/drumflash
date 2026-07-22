//! Pattern bank bar: Export/Drag chips, Save/Load slots P1-P8, Clear.

use crate::ui::controls::chip_button;
use crate::ui::editor_state::EditorUIState;
use crate::ui::midi::{export_midi_to_documents, start_external_midi_drag};
use crate::ui::theme::*;
use crate::{
    sequencer::{Pattern, SharedPattern},
    DrumFlashParams,
};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use std::sync::{
    atomic::AtomicU32,
    Arc,
};

pub fn draw_pattern_bank(
    ui: &mut egui::Ui,
    state: &mut EditorUIState,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    save_pattern_request: &Arc<AtomicU32>,
    load_pattern_request: &Arc<AtomicU32>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Patterns").strong().size(12.0).color(INK()));
        ui.add_space(8.0);

        // MIDI export chips (left side, always visible)
        if chip_button(ui, "Export", false, BLUE(), egui::Sense::click()).clicked() {
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
        let drag_response = chip_button(ui, "Drag", false, BLUE(), egui::Sense::click())
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
            let blue = BLUE();
            let blink = blink as f32;
            Color32::from_rgb(
                (blue.r() as f32 + blink * 80.0) as u8,
                (blue.g() as f32 + blink * 40.0) as u8,
                blue.b(),
            )
        } else {
            PANEL2()
        };
        let save_btn = egui::Button::new(RichText::new("Save").size(10.0).strong().monospace())
            .min_size(Vec2::new(44.0, 26.0))
            .fill(save_fill)
            .stroke(egui::Stroke::new(
                1.5,
                if is_save_mode { BLUE() } else { LINE2() },
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

            let btn_size = Vec2::new(30.0, 26.0);
            let fill = if is_loaded {
                P_ACTIVE()
            } else if occupied {
                PANEL2()
            } else {
                BG() // much darker for empty slot
            };
            let stroke_color = if is_loaded {
                GREEN() // green ring for loaded
            } else if occupied {
                LINE2()
            } else {
                LINE() // dimmer border for empty slot
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

                        let label_color = if is_loaded { GREEN() } else { INK() };
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
                        drop(bank_mut);
                        params.pattern_bank.refresh_snapshot();
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
            let d = DANGER();
            let blink = clear_blink as f32;
            Color32::from_rgb(
                (d.r() as f32 - 55.0 + blink * 55.0) as u8,
                (d.g() as f32 - 20.0 + blink * 40.0) as u8,
                (d.b() as f32 - 20.0 + blink * 40.0) as u8,
            )
        } else {
            PANEL2()
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
                DANGER_SOFT()
            } else {
                LINE2()
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
                    .color(RED()),
            );
        }
    });
}

/// Capture the current pattern, plocks and fusions into the given pattern-bank slot.
pub fn save_current_pattern_to_bank_slot(
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    slot: usize,
) {
    if slot >= crate::pattern_bank::SLOT_COUNT {
        return;
    }
    let Ok(mut bank) = params.pattern_bank.bank.lock() else {
        return;
    };
    let pattern_length = params.pattern_length.value() as u8;
    bank.slots[slot].capture(
        pattern,
        &params.plock_state.state,
        &params.seq_plock_state.state,
        pattern_length,
    );
    drop(bank);
    params.pattern_bank.refresh_snapshot();
}

pub fn load_pattern_for_ui(pattern_for_ui: &SharedPattern, pattern: &Pattern) {
    let masks = pattern.step_masks();
    pattern_for_ui.load_step_masks(&masks);
    for inst in 0..crate::sequencer::pattern::INSTRUMENT_COUNT {
        pattern_for_ui.store_fusions(inst, &pattern.fusions[inst]);
    }
}

pub fn load_pattern_for_ui_with_length(
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
