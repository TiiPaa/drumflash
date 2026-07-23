//! Popups: Add Module picker, page menu, global settings, lane preset warning.

use crate::plock::PlockState;
use crate::sequencer::SharedPattern;
use crate::sound_settings::SoundSettingsState;
use crate::track::{TrackInstrumentKind, TrackLayoutState};
use crate::ui::editor_state::{
    EditorUIState, LanePresetAction, PageMenuAction,
};
use crate::ui::local_param_slider::LocalParamSlider;
use crate::ui::menus::{page_menu_frame, page_menu_header, plock_menu_action_row};
use crate::ui::sound_editor::apply_lane_preset_action;
use crate::ui::theme;
use crate::ui::theme::*;
use crate::ui::widgets::styled_select;
use crate::DrumFlashParams;
use nih_plug::{params::persist::PersistentField, prelude::*};
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};

pub fn draw_add_module_popup_if_any(
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
            page_menu_frame(ui, BLUE(), |ui| {
                page_menu_header(ui, &format!("Slot {} - Add Module", popup.slot + 1), BLUE());
                for kind_idx in 0..TrackInstrumentKind::COUNT {
                    let Some(kind) = TrackInstrumentKind::from_index(kind_idx) else {
                        continue;
                    };
                    if plock_menu_action_row(ui, kind.default_name(), BLUE()).clicked() {
                        crate::ui::grid::activate_slot(params, sound_settings, state, popup.slot, kind);
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
                    ui.add_space((ui.available_width() - 20.0).max(0.0));
                    if ui
                        .button(
                            RichText::new("x")
                                .font(f_sans_med(10.5))
                                .color(INK3()),
                        )
                        .clicked()
                    {
                        state.settings_open = false;
                    }
                });
                ui.add_space(12.0);

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

pub fn draw_lane_preset_dropdown(ui: &mut egui::Ui, state: &mut EditorUIState) {
    egui::ComboBox::from_id_salt("lane_preset_dropdown")
        .selected_text(RichText::new("Preset").font(f_sans_sb(10.5)).color(INK2()))
        .width(94.0)
        .show_ui(ui, |ui| {
            ui.set_min_width(132.0);
            if ui
                .selectable_label(
                    false,
                    RichText::new("Clear All").font(f_sans_med(11.0)).color(RED()),
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

pub fn draw_lane_preset_warning_if_any(
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
                .fill(P_ACTIVE())
                .corner_radius(RADIUS_PANEL)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(panel_w);
                    ui.label(RichText::new("Warning").font(f_sans_sb(12.0)).color(RED()));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "{} modifies the current pattern and lane layout.",
                            action.label()
                        ))
                        .font(f_sans_med(10.5))
                        .color(INK2()),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let apply = egui::Button::new(
                            RichText::new(action.apply_label())
                                .font(f_sans_sb(10.5))
                                .color(Color32::WHITE),
                        )
                        .min_size(Vec2::new(128.0, CTL_HEIGHT))
                        .fill(RED())
                        .stroke(egui::Stroke::new(1.0, RED()))
                        .corner_radius(RADIUS_CTL);
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
                            RichText::new("Cancel").font(f_sans_sb(10.5)).color(INK2()),
                        )
                        .min_size(Vec2::new(82.0, CTL_HEIGHT))
                        .fill(PANEL2())
                        .stroke(egui::Stroke::new(1.0, LINE2()))
                        .corner_radius(RADIUS_CTL);
                        if ui.add(cancel).clicked() {
                            state.lane_preset_confirm = None;
                        }
                    });
                });
        });
}
