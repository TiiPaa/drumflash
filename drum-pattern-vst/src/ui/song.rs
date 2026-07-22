//! Song editor: 16 song blocks (pattern selector + repeat), clear all.

use crate::pattern_bank::{SLOT_COUNT, SONG_BLOCKS};
use crate::ui::editor_state::EditorUIState;
use crate::ui::theme::*;
use crate::DrumFlashParams;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

pub fn draw_song_editor(
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
                .fill(DANGER_DIM())
                .stroke(egui::Stroke::new(1.0, LINE2()))
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
                .fill(PANEL2())
                .stroke(egui::Stroke::new(1.0, LINE2()))
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
                BLUE()
            } else if occupied {
                PANEL2()
            } else {
                SONG_EMPTY()
            };
            let stroke_color = if is_selected {
                BLUE()
            } else if is_current {
                BLUE()
            } else {
                LINE2()
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

    // Publish any song change to the audio-thread lock-free controller.
    let current_song = bank.song;
    let song_changed = state.last_published_song != Some(current_song);
    drop(bank);
    if song_changed {
        params.pattern_bank.refresh_snapshot();
        params.song_controller.publish(current_song);
        state.last_published_song = Some(current_song);
    }
}
