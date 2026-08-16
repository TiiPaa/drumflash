//! Popups: Add Module picker, page menu, global settings.

use crate::plock::PlockState;
use crate::sequencer::SharedPattern;
use crate::track::TrackLayoutState;
use crate::ui::editor_state::{EditorUIState, PageMenuAction};
use crate::ui::local_param_slider::LocalParamSlider;
use crate::ui::menus::{page_menu_frame, page_menu_header, plock_menu_action_row};
use crate::ui::theme;
use crate::ui::theme::*;
use crate::ui::widgets::styled_select;
use crate::DrumFlashParams;
use nih_plug::{params::persist::PersistentField, prelude::*};
use nih_plug_egui::egui::{self, RichText, Vec2};

pub fn draw_page_popup_if_any(
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
    let accent = BLUE();

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
                        Some(crate::ui::grid::copy_page_to_clipboard(pattern, plock, params, page));
                    state.page_popup = None;
                }

                if confirm_action == Some(PageMenuAction::Paste) {
                    ui.label(
                        RichText::new("Overwrite page?")
                            .font(f_sans_med(10.0))
                            .color(INK3()),
                    );
                    if plock_menu_action_row(ui, "Yes, overwrite", PL_LINK()).clicked() {
                        if let Some(ref clipboard) = state.page_clipboard {
                            crate::ui::grid::paste_page_from_clipboard(
                                pattern, plock, params, page, clipboard,
                            );
                            // Auto-extend pattern length so the pasted page is actually played.
                            let required_len = ((page + 1) * 16).clamp(1, 64) as i32;
                            let current_len = params.pattern_length.value().clamp(1, 64) as i32;
                            if required_len > current_len {
                                setter.set_parameter(&params.pattern_length, required_len);
                            }
                        }
                        state.page_popup = None;
                    }
                    if plock_menu_action_row(ui, "No, cancel", INK3()).clicked() {
                        state.page_popup = None;
                    }
                } else {
                    let paste_enabled = has_clipboard;
                    let paste_color = if paste_enabled { PL_LINK() } else { INK3() };
                    if plock_menu_action_row(ui, "Paste", paste_color).clicked() && paste_enabled {
                        popup.confirm_action = Some(PageMenuAction::Paste);
                        state.page_popup = Some(popup);
                    }
                }

                if confirm_action == Some(PageMenuAction::Clear) {
                    ui.label(
                        RichText::new("Clear page?")
                            .font(f_sans_med(10.0))
                            .color(INK3()),
                    );
                    if plock_menu_action_row(ui, "Yes, clear", RED()).clicked() {
                        crate::ui::grid::clear_page_for_ui(pattern, plock, params, page);
                        state.page_popup = None;
                    }
                    if plock_menu_action_row(ui, "No, cancel", INK3()).clicked() {
                        state.page_popup = None;
                    }
                } else {
                    if plock_menu_action_row(ui, "Clear", RED()).clicked() {
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

/// Global settings popup (default analog value, MIDI settings, skin).
pub fn draw_settings_popup_if_any(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    state: &mut EditorUIState,
) {
    if !state.settings_open {
        return;
    }

    let anchor = ui.max_rect().right_top() + Vec2::new(-230.0, 60.0);
    let area_id = ui.id().with("settings_popup");

    let response = egui::Area::new(area_id)
        .kind(egui::UiKind::Popup)
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(ui.ctx(), |ui| {
            page_menu_frame(ui, BLUE(), |ui| {
                ui.set_min_width(200.0);
                ui.set_max_width(220.0);

                // Header with close button
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Settings").font(f_sans_sb(11.0)).color(BLUE()));
                    ui.add_space((ui.available_width() - 22.0).max(0.0));
                    if crate::ui::controls::keycap_button(
                        ui,
                        "×",
                        22.0,
                        crate::ui::widgets::KeycapState::Rest,
                        true,
                        f_sans_med(12.0),
                    )
                    .clicked()
                    {
                        state.settings_open = false;
                    }
                });
                ui.add_space(12.0);

                // Auto-Edit (moved here from the header).
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Auto-Edit").font(f_sans_med(10.5)).color(INK3()));
                    ui.add_space((ui.available_width() - 34.0).max(0.0));
                    let checked = params.auto_edit.value();
                    if ui.add(crate::ui::widgets::ToggleSwitch::new(checked)).clicked() {
                        crate::ui::controls::set_bool_param_if_changed(setter, &params.auto_edit, !checked);
                    }
                });
                ui.add_space(14.0);

                // Default Analog
                ui.label(
                    RichText::new("Default Analog")
                        .font(f_sans_med(10.5))
                        .color(INK3()),
                );
                ui.add_space(4.0);
                let mut value = state.global_config.default_analog;
                let slider =
                    LocalParamSlider::new(&mut value, 0.0..=1.0).reset_value(0.5);
                if ui.add(slider).changed() {
                    state.global_config.default_analog = value.clamp(0.0, 1.0);
                    let _ = state.global_config.save();
                }

                ui.add_space(12.0);

                // Global MIDI Channel
                ui.label(
                    RichText::new("Global MIDI Channel")
                        .font(f_sans_med(10.5))
                        .color(INK3()),
                );
                ui.add_space(4.0);
                let mut channel = state.global_config.global_midi_channel as i32;
                if ui
                    .add(egui::DragValue::new(&mut channel).range(1..=16).speed(1.0))
                    .changed()
                {
                    let channel = channel.clamp(1, 16) as u8;
                    state.global_config.global_midi_channel = channel;
                    let _ = state.global_config.save();
                    // Also update the current track layout so the change is heard
                    // immediately without reloading the project.
                    let mut layout = PersistentField::<TrackLayoutState>::map(&params.track_layout, |s| s.clone());
                    layout.global_midi_channel = channel;
                    PersistentField::<TrackLayoutState>::set(&params.track_layout, layout);
                }

                ui.add_space(12.0);

                // Skin selector
                ui.label(
                    RichText::new("Skin")
                        .font(f_sans_med(10.5))
                        .color(INK3()),
                );
                ui.add_space(4.0);
                let skin_names: Vec<&str> =
                    theme::SKINS.iter().map(|(name, _)| *name).collect();
                let current_idx = skin_names
                    .iter()
                    .position(|n| *n == theme::skin_name())
                    .unwrap_or(0);
                let (response, picked) =
                    styled_select(ui, "settings_skin", current_idx, &skin_names, 120.0);
                let _ = response;
                if let Some(idx) = picked {
                    let name = skin_names[idx.min(skin_names.len().saturating_sub(1))];
                    theme::set_skin(name);
                    state.global_config.skin = name.to_string();
                    let _ = state.global_config.save();
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
        state.settings_open = false;
    }
}

pub fn draw_pattern_load_warning_if_any(
    ui: &mut egui::Ui,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
    load_pattern_request: &std::sync::Arc<std::sync::atomic::AtomicU32>,
) {
    let Some(slot) = state.pattern_load_confirm else {
        return;
    };

    let screen_rect = ui.ctx().screen_rect();
    let panel_w = 338.0;
    let pos = egui::pos2(
        screen_rect.center().x - panel_w * 0.5,
        screen_rect.center().y - 40.0,
    );
    egui::Area::new(ui.id().with("pattern_load_warning"))
        .kind(egui::UiKind::Popup)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ui.ctx(), |ui| {
            let bg = ui.painter().add(egui::Shape::Noop);
            let resp = egui::Frame::NONE
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(panel_w);
                    ui.label(RichText::new("Warning").font(f_sans_sb(12.0)).color(RED()));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "The current pattern has unsaved changes. Switching to P{} will discard them.",
                            slot + 1
                        ))
                        .font(f_sans_med(10.5))
                        .color(INK2()),
                    );
                    ui.add_space(10.0);
                    // Shared tail of the two confirming actions: switch to the
                    // target slot (loading its pattern, or a fresh empty grid).
                    let switch_to_slot = |state: &mut EditorUIState| {
                        let occupied = params
                            .pattern_bank
                            .bank
                            .lock()
                            .map(|b| b.slots[slot].occupied)
                            .unwrap_or(false);
                        if occupied {
                            load_pattern_request
                                .store((slot + 1) as u32, std::sync::atomic::Ordering::Relaxed);
                        } else {
                            crate::ui::pattern_bank::clear_current_grid(pattern, params);
                        }
                        state.last_loaded_slot = Some(slot);
                        state.pattern_load_confirm = None;
                    };
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        // Save the current pattern into ITS slot first, then switch.
                        if let Some(current) = state.last_loaded_slot {
                            if crate::ui::controls::chip_button(
                                ui,
                                "Save & Load",
                                true,
                                BLUE(),
                                egui::Sense::click(),
                            )
                            .clicked()
                            {
                                crate::ui::pattern_bank::save_current_pattern_to_bank_slot(
                                    params, pattern, current,
                                );
                                switch_to_slot(state);
                            }
                        }
                        if crate::ui::controls::chip_button(
                            ui,
                            format!("Discard & Load P{}", slot + 1).as_str(),
                            true,
                            RED(),
                            egui::Sense::click(),
                        )
                        .clicked()
                        {
                            switch_to_slot(state);
                        }
                        if crate::ui::controls::chip_button(ui, "Cancel", false, INK2(), egui::Sense::click())
                            .clicked()
                        {
                            state.pattern_load_confirm = None;
                        }
                    });
                });
            ui.painter()
                .set(bg, crate::ui::skeuo::plate_shape(resp.response.rect, RADIUS_PANEL as f32));
        });
}
