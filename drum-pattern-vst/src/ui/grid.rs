//! Pattern grid: lanes, step cells, fusion, lane reorder, page bar.

use crate::plock::PlockState;
use crate::sequencer::{
    FusedGroup, SharedPattern,
};
use crate::sound_settings::SoundSettingsState;
use crate::track::{TrackInstrumentKind, TrackLayoutState, TrackSlot};
use crate::ui::controls::*;
use crate::ui::editor_state::*;
use crate::ui::header::header_param_slider;
use crate::ui::popups::{
    draw_add_module_popup_if_any, draw_lane_preset_dropdown, draw_lane_preset_warning_if_any,
    draw_page_popup_if_any,
};
use crate::ui::slider;
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::DrumFlashParams;
use nih_plug::{params::persist::PersistentField, prelude::*};
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
};

pub fn draw_grid_v2(
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
    // When the p-lock popup is open, any primary click this frame (menu item,
    // popup border, or outside) should be handled by the popup, not by the step
    // cell underneath it. Suppress step-cell toggles for this frame.
    state.suppress_step_cell_click = false;
    if state.plock_popup.is_some()
        && ui
            .input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
    {
        state.suppress_step_cell_click = true;
    }

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

    let well_resp = egui::Frame::new()
        .fill(WELL_FILL)
        .stroke(egui::Stroke::new(1.0, PANEL_BORDER))
        .corner_radius(RADIUS_PANEL)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = GAP_TIGHT;

            let row_w = ui.available_width();
            let grip_w = 14.0;
            let name_w = 50.0;
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

    // Recessed well: dark inset shadow along the top edge + faint bottom
    // highlight (SPEC seqwrap: inset 0 2 6 rgba(0,0,0,.6) + inset 0 -1 0 white .05).
    {
        let wr = well_resp.response.rect;
        let p = ui.painter_at(wr);
        let x0 = wr.left() + 4.0;
        let x1 = wr.right() - 4.0;
        for (dy, a) in [(1.0f32, 150u8), (2.0, 95), (3.0, 55), (4.0, 30), (5.0, 14)] {
            p.line_segment(
                [egui::pos2(x0, wr.top() + dy), egui::pos2(x1, wr.top() + dy)],
                egui::Stroke::new(1.0, Color32::from_black_alpha(a)),
            );
        }
    }

    let mut fusion_edit_box_rect = None;
    ui.horizontal(|ui| {
        ui.set_height(28.0);
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.label(
            RichText::new("P-Lock Mode")
                .font(f_sans_sb(10.5))
                .color(INK3()),
        );
        let selected = if state.sequencer_mode { 1 } else { 0 };
        let new_selected = p_lock_mode_segmented(ui, selected);
        if new_selected != selected {
            state.sequencer_mode = new_selected == 1;
        }
        ui.label(
            RichText::new("Right-click a step to edit its p-lock")
                .size(10.5)
                .color(INK3()),
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
        let layout_state =
            PersistentField::<TrackLayoutState>::map(&params.track_layout, |s| s.clone());
        let slot_name = {
            let name = if layout_state.slots[slot_idx].name.is_empty() {
                crate::instrument_registry::INSTRUMENTS[voice_idx]
                    .label
                    .to_string()
            } else {
                layout_state.slots[slot_idx].name.clone()
            };
            name.chars().take(6).collect::<String>()
        };
        let name_response = draw_lane_name_v2(ui, name_w, selected, &slot_name)
            .on_hover_text(crate::instrument_registry::INSTRUMENTS[voice_idx].full_name);
        if name_response.clicked() {
            select_legacy_track(state, slot_idx);
        }
        if name_response.double_clicked() {
            select_legacy_track(state, slot_idx);
            state.sound_editor_tab = SoundEditorTab::Track;
            state.track_name_input = layout_state.slots[slot_idx].name.clone();
            state.track_name_input_slot = Some(slot_idx);
            state.track_name_focus_request = true;
        }

        name_response.context_menu(|ui| {
            if ui.button("Copy Lane").clicked() {
                state.copy_lane(params, slot_idx, sound_settings, pattern, plock);
                state.lane_clear_grid_confirm = None;
                state.lane_delete_confirm = None;
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
                state.lane_delete_confirm = None;
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
                state.lane_delete_confirm = None;
                ui.close_menu();
            }
            ui.separator();
            let confirm_clear_grid = state.lane_clear_grid_confirm == Some(slot_idx);
            if ui
                .button(
                    RichText::new(if confirm_clear_grid {
                        "Confirm Clear Lane?"
                    } else {
                        "Clear Lane"
                    })
                    .font(f_sans_med(11.0))
                    .color(RED()),
                )
                .on_hover_text(if confirm_clear_grid {
                    "Click again to clear this lane's steps, fusions and plocks"
                } else {
                    "Clear this lane's steps, fusions and plocks; keeps instrument, sound, routing and lane controls"
                })
                .clicked()
            {
                if confirm_clear_grid {
                    state.clear_lane(params, slot_idx, pattern, plock);
                    ui.close_menu();
                } else {
                    state.lane_clear_grid_confirm = Some(slot_idx);
                    state.lane_delete_confirm = None;
                }
            }
            let confirm_delete = state.lane_delete_confirm == Some(slot_idx);
            if ui
                .button(
                    RichText::new(if confirm_delete {
                        "Confirm Delete Lane?"
                    } else {
                        "Delete Lane"
                    })
                    .font(f_sans_med(11.0))
                    .color(RED()),
                )
                .on_hover_text("Deactivate this lane; the slot becomes empty and can be reactivated later")
                .clicked()
            {
                if confirm_delete {
                    deactivate_slot(params, state, slot_idx);
                    ui.close_menu();
                } else {
                    state.lane_delete_confirm = Some(slot_idx);
                    state.lane_clear_grid_confirm = None;
                }
            }
            if ui
                .button(RichText::new("Randomize Lane").font(f_sans_med(11.0)))
                .on_hover_text("Fill this lane with random steps (30% density); clears fusions and plocks")
                .clicked()
            {
                state.randomize_lane(params, slot_idx, pattern, plock);
                // Flash visual feedback
                state.slot_flash_until[slot_idx] = ui.ctx().input(|i| i.time) + 0.5;
                state.lane_delete_confirm = None;
                ui.close_menu();
            }
        });

        let inst_state = &sound_settings.instruments[slot_idx];
        let mut lane_vol = f32::from_bits(inst_state.volume.load(Ordering::Relaxed));
        let lane_vol_response =
            draw_mini_value_slider(ui, &mut lane_vol, 0.0, 2.0, 1.0, vol_w, BLUE(), "Lane Volume");
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
                AMBER(),
                MUTE_FILL(),
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
                GREEN(),
                SOLO_FILL(),
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
            if draw_tag_button_v2(ui, "T", AMBER(), Color32::BLACK, is_flashing, "Test").clicked() {
                voice_test_triggers[slot_idx].store(true, Ordering::Release);
                if params.auto_edit.value() {
                    select_legacy_track(state, slot_idx);
                }
            }
        });

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP_TIGHT;

            // Long-press drag: update target, apply move on release, and activate
            // after the ~0.5 s hold threshold.
            let mut drag_just_completed = false;
            if let Some(drag) = state.step_drag.as_mut() {
                if drag.slot == slot_idx {
                    if !drag.active {
                        let now = ui.ctx().input(|i| i.time);
                        if now - drag.start_time >= 0.5 {
                            drag.active = true;
                        }
                    }
                    if drag.active {
                        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                            let delta_x = pos.x - drag.source_rect.center().x;
                            let delta_steps = (delta_x / (cell_w + GAP_TIGHT)).round() as i32;
                            let min_step = page_offset as i32;
                            let max_step = (page_offset + 15).min(crate::plock::STEP_COUNT - 1) as i32;
                            let target =
                                (drag.source_step as i32 + delta_steps).clamp(min_step, max_step)
                                    as usize;
                            drag.current_target = target;
                        }
                        if ui.input(|i| i.pointer.any_released()) {
                            if drag.source_step != drag.current_target {
                                move_step_with_plocks(
                                    pattern,
                                    plock,
                                    params,
                                    slot_idx,
                                    drag.source_step,
                                    drag.current_target,
                                );
                                state.mark_pattern_dirty();
                            }
                            state.step_drag = None;
                            drag_just_completed = true;
                        }
                    }
                }
            }

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
                let has_sound_plock_base = !beyond_len && plock.masks.is_active(slot_idx, source_step);
                let has_morph = !beyond_len
                    && fusion_group.map(|g| g.morph_active()).unwrap_or(false);
                let has_sound_plock = has_sound_plock_base || has_morph;
                let field_mask = if has_sound_plock_base {
                    plock.field_masks.get(slot_idx, source_step)
                } else {
                    0
                };
                let all_bits = (1u64 << crate::plock::FIELD_COUNT) - 1;
                let is_snapshot = has_sound_plock_base && field_mask == all_bits;
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

                let is_drag_source = state.step_drag.as_ref().map_or(false, |d| {
                    d.active && d.slot == slot_idx && d.source_step == global_step
                });
                let is_drag_target = state.step_drag.as_ref().map_or(false, |d| {
                    d.active && d.slot == slot_idx && d.current_target == global_step
                });

                let (fill, mut stroke) = step_colors_v2(
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
                if is_drag_target {
                    stroke = egui::Stroke::new(2.0, DRAG_TARGET());
                }
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

                // Start a long-press drag when the left button is held on an active,
                // single, non-fused cell outside fusion mode.
                if !beyond_len
                    && active
                    && !fusion_mode_active
                    && fusion_group.is_none()
                    && response.is_pointer_button_down_on()
                {
                    if state.step_drag.is_none() {
                        let now = ui.ctx().input(|i| i.time);
                        state.step_drag = Some(DragStepState {
                            slot: slot_idx,
                            source_step: global_step,
                            source_rect: response.rect,
                            start_time: now,
                            active: false,
                            current_target: global_step,
                        });
                    }
                }

                let drag_active = state.step_drag.as_ref().map_or(false, |d| d.active);
                let suppress_click = drag_active || drag_just_completed || state.suppress_step_cell_click || state.plock_popup.is_some();

                if is_drag_target {
                    ui.painter().rect_filled(
                        response.rect,
                        4.0,
                        Color32::from_rgba_unmultiplied(255, 200, 80, 50),
                    );
                }
                if is_drag_source {
                    let t = ui.ctx().input(|i| i.time) as f32;
                    let pulse = ((t * 8.0).sin() + 1.0) * 0.5;
                    let alpha = (140.0 + 115.0 * pulse) as u8;
                    let fill_alpha = (50.0 + 50.0 * pulse) as u8;
                    ui.painter().rect_filled(
                        response.rect,
                        4.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, fill_alpha),
                    );
                    ui.painter().rect_stroke(
                        response.rect.shrink(1.0),
                        4.0,
                        egui::Stroke::new(2.5, Color32::from_rgba_unmultiplied(255, 255, 255, alpha)),
                        egui::StrokeKind::Inside,
                    );
                }

                if !beyond_len && !suppress_click && response.double_clicked() {
                    if let Some((idx, _)) = fusion_info {
                        select_legacy_track(state, slot_idx);
                        state.fusion_editing = Some((slot_idx, idx));
                        state.fusion_edit_focus_request = true;
                        state.fusion_edit_steps = fusion_group.map(|g| g.step_count).unwrap_or(1);
                        *fusion_editing_started_this_frame = true;
                    }
                } else if !beyond_len && !suppress_click && response.clicked_by(egui::PointerButton::Primary) && fusion_mode_active {
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
                } else if !beyond_len && !suppress_click && response.clicked_by(egui::PointerButton::Primary) {
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
                    state.mark_pattern_dirty();
                    if params.auto_edit.value() {
                        select_legacy_track(state, slot_idx);
                    }
                    state.fusion_selection_start[slot_idx] = None;
                }

                if !beyond_len && response.secondary_clicked() {
                    // Plock editing is only allowed in Pattern mode with grid Follow OFF.
                    // Song mode switches patterns automatically and Follow ON scrolls pages,
                    // both of which make the grid unstable under the cursor.
                    if !params.song_mode.value() && !state.follow_mode {
                        select_legacy_track(state, slot_idx);
                        if let Some(pos) = response.interact_pointer_pos() {
                            state.plock_popup = Some(PlockPopup {
                                instrument: slot_idx,
                                step: source_step,
                                step_was_active: active,
                                screen_pos: pos,
                                morph_menu: false,
                            });
                        }
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
            BLUE(),
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
            BLUE(),
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
    let fill = if response.hovered() { P_HOVER() } else { PANEL2() };
    ui.painter().rect_filled(rect, RADIUS_PAD, fill);
    ui.painter().rect_stroke(
        rect,
        RADIUS_PAD,
        egui::Stroke::new(1.0, LINE2()),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("+{}", slot_number),
        f_mono_sb(11.0),
        FAINT(),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn draw_empty_lane_chip_v2(ui: &mut egui::Ui, width: f32, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 21.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, RADIUS_CTL, BG());
    ui.painter().rect_stroke(
        rect,
        RADIUS_CTL,
        egui::Stroke::new(1.0, LINE()),
        egui::StrokeKind::Inside,
    );
    if !label.is_empty() {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            f_sans_med(9.5),
            FAINT(),
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
        egui::Stroke::new(2.0, BLUE()),
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
pub fn activate_slot(
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
    sound_settings.reset_slot_to_defaults(slot_idx, kind, state.global_config.default_analog);
    select_legacy_track(state, slot_idx);
}

/// Deactivate an active slot so it becomes an empty lane again.
/// Triggered from the lane title context menu.
fn deactivate_slot(params: &DrumFlashParams, state: &mut EditorUIState, slot_idx: usize) {
    if slot_idx >= crate::track::MAX_TRACKS {
        return;
    }
    let mut new_state =
        PersistentField::<TrackLayoutState>::map(&params.track_layout, |s| s.clone());
    if !new_state.slots[slot_idx].active {
        return;
    }
    new_state.slots[slot_idx].active = false;
    let next_selected = new_state.active_slot_indices().next().unwrap_or(0);
    PersistentField::<TrackLayoutState>::set(&params.track_layout, new_state);

    state.fusion_selection_start[slot_idx] = None;
    state.fusion_editing = state.fusion_editing.filter(|(idx, _)| *idx != slot_idx);
    state.plock_popup = state
        .plock_popup
        .filter(|popup| popup.instrument != slot_idx);
    state.lane_clear_grid_confirm = None;
    state.lane_delete_confirm = None;
    state.add_module_popup = state
        .add_module_popup
        .filter(|popup| popup.slot != slot_idx);

    if state.selected_track_slot == slot_idx {
        state.selected_track_slot = next_selected;
        state.selected_instrument = next_selected;
    }
}

/// Instrument picker popup for an empty lane (opened by the `+N` chip).

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
                ui.label(RichText::new("Page").font(f_sans_sb(10.5)).color(INK3()));
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
                .fill(if active { BLUE() } else { PANEL2() })
                .stroke(egui::Stroke::new(1.0, if active { BLUE() } else { LINE2() }))
                .corner_radius(RADIUS_CTL),
            );
            if play_page == page {
                let led = response.rect.center_bottom() + egui::vec2(0.0, 6.0);
                ui.painter().circle_filled(
                    led,
                    5.0,
                    Color32::from_rgba_unmultiplied(248, 113, 113, 45),
                );
                ui.painter().circle_filled(led, 2.5, RED());
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
                INK2()
            }),
        )
        .min_size(Vec2::new(78.0, CTL_HEIGHT))
        .fill(if state.follow_mode { BLUE() } else { PANEL2() })
        .stroke(egui::Stroke::new(
            1.0,
            if state.follow_mode { BLUE() } else { LINE2() },
        ))
        .corner_radius(RADIUS_CTL);
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
                ui.label(RichText::new("Len").font(f_sans_sb(10.5)).color(INK3()));
                header_param_slider(ui, setter, &params.pattern_length, 132.0, "", false);
                draw_len_value_fixed(ui, master_length, LEN_VALUE_W);
                for &len in &[16, 32, 48, 64] {
                    let active = master_length == len;
                    let btn =
                        egui::Button::new(RichText::new(format!("{}", len)).monospace().size(10.5))
                            .min_size(Vec2::new(36.0, CTL_HEIGHT))
                            .fill(if active { BLUE() } else { PANEL2() })
                            .stroke(egui::Stroke::new(1.0, if active { BLUE() } else { LINE2() }))
                            .corner_radius(RADIUS_CTL);
                    if ui.add(btn).clicked() {
                        setter.set_parameter(&params.pattern_length, len as i32);
                    }
                }
                let can_double = master_length <= 32;
                let x2 = egui::Button::new(RichText::new("x2").monospace().size(10.5))
                    .min_size(Vec2::new(36.0, CTL_HEIGHT))
                    .fill(PANEL2())
                    .stroke(egui::Stroke::new(1.0, LINE2()))
                    .corner_radius(RADIUS_CTL);
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


fn draw_len_value_fixed(ui: &mut egui::Ui, master_length: usize, width: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, CTL_HEIGHT), egui::Sense::hover());
    let number_text = format!("{:>2}", master_length);
    let number_pos = egui::pos2(rect.left(), rect.center().y);
    ui.painter().text(
        number_pos,
        egui::Align2::LEFT_CENTER,
        number_text,
        f_mono(12.0),
        INK(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 23.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        "steps",
        f_sans(9.5),
        INK3(),
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
            egui::Label::new(RichText::new("Vol").font(f_sans_sb(9.5)).color(INK3())),
        );
        // M / S / T column headings (aligned with the lane tags below)
        ui.allocate_ui(Vec2::new(mst_w, 16.0), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = GAP_TIGHT;
                for t in ["M", "S", "T"] {
                    ui.add_sized(
                        Vec2::new(STEP_H, 16.0),
                        egui::Label::new(RichText::new(t).font(f_mono(9.0)).color(INK3())),
                    );
                }
            });
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP_TIGHT;
            for local in 0..16 {
                let step = page_offset + local;
                let color = if play_step == step {
                    BLUE()
                } else if local % 4 == 0 {
                    INK2()
                } else {
                    FAINT()
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
                egui::Label::new(RichText::new(label).font(f_sans_sb(9.5)).color(INK3())),
            );
        }
    });
}

fn draw_seq_grip_v2(ui: &mut egui::Ui, width: f32, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::drag());
    // 2x3 dot matrix (drag-handle look; avoids relying on braille glyph coverage)
    let c = rect.center();
    let dot_color = if response.dragged() || response.hovered() {
        INK2()
    } else {
        FAINT()
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
        BLUE()
    } else if response.hovered() {
        P_HOVER()
    } else {
        PANEL2()
    };
    ui.painter().rect_filled(rect, RADIUS_PAD, fill);
    // Borderless at rest; only the selected lane gets a blue outline.
    if selected {
        ui.painter().rect_stroke(
            rect,
            RADIUS_PAD,
            egui::Stroke::new(1.0, BLUE()),
            egui::StrokeKind::Inside,
        );
    }
    let text_color = if selected {
        Color32::WHITE
    } else if response.hovered() {
        INK()
    } else {
        INK2()
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
    let fill = if active { color } else { PANEL2() };
    let text = if active { text_on } else { FAINT() };
    ui.painter().rect_filled(rect, RADIUS_TAG, fill);
    ui.painter().rect_stroke(
        rect,
        RADIUS_TAG,
        egui::Stroke::new(1.0, if active { color } else { LINE2() }),
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

    let hover = hover_t(ui.ctx(), response.id, response.hovered() && enabled && !is_fusion_mid);
    let stroke = if hover > 0.01 {
        egui::Stroke::new(1.0, lerp_color(stroke.color, BLUE(), hover))
    } else {
        stroke
    };

    if !is_fusion_mid {
        if dashed_border {
            // Empty-lane placeholder: flat recessed fill + dotted outline.
            ui.painter().rect_filled(block_rect, 4.0, fill);
            if stroke.width > 0.0 && stroke.color.a() > 0 {
                draw_dashed_rect(ui.painter(), block_rect.shrink(0.5));
            }
        } else {
            // Skeuo pad texture (designer recipe, rust/skeuo_widgets.rs::pad): an
            // Image rounded to the pad radius via corner_radius. Fusion blocks /
            // editing / selection keep the vector fallback.
            let source = if fusion_span.is_none() {
                crate::ui::pads::pad_source_for(fill)
            } else {
                None
            };
            if let Some(src) = source {
                // egui can't round a texture in this renderer, so the pad shows
                // the PNG's own baked corner (~2 px at cell size). Overlays below
                // use RADIUS_PAD_TEX to match it.
                egui::Image::new(src)
                    .tint(Color32::WHITE)
                    .paint_at(ui, block_rect);
            } else {
                paint_pad(ui.painter(), block_rect, fill);
            }
            // Hover highlight, hugging the pad's real (baked) corner.
            if hover > 0.01 {
                ui.painter().rect_stroke(
                    block_rect,
                    egui::epaint::CornerRadius::same(RADIUS_PAD_TEX as u8),
                    egui::Stroke::new(1.0, lerp_color(stroke.color, BLUE(), hover)),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    // Playhead: an inset white ring drawn ON TOP of the state border, with a
    // subtle pulse so the playing column reads at a glance.
    if playhead {
        let time = ui.ctx().input(|i| i.time) as f32;
        let pulse = ((time * 2.5).sin() + 1.0) * 0.5;
        let alpha = 120 + (pulse * 80.0) as u8;
        ui.painter().rect_stroke(
            rect.shrink(0.75),
            egui::epaint::CornerRadius::same(RADIUS_PAD_TEX as u8),
            egui::Stroke::new(1.5, white_a(alpha)),
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

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// Fill a rect with a vertical (top→bottom) gradient via a 2-triangle Mesh.
fn fill_vgrad(painter: &egui::Painter, rect: egui::Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// Paint one step cell as a skeuomorphic rubber pad — exact recipe ported from
/// the designer's `fd-skeuo.css` (`.step` variants). Off pads are a vertical
/// gradient; lit pads are a radial (concentric circles at 50%/30%) from a bright
/// core, through the state colour, to a darkened rim. SKEUO.md §1-2.
fn paint_pad(painter: &egui::Painter, rect: egui::Rect, fill: Color32) {
    let r = 4.0;

    // Map the incoming state colour to the exact skeuo recipe.
    // Lit recipes: (core, mid, outer, border, glow-rgb).
    let lit = if fill == BLUE() {
        Some((rgb(154, 220, 255), rgb(74, 182, 255), rgb(30, 110, 160), rgb(13, 58, 92)))
    } else if fill == PL_LINK() {
        Some((rgb(255, 217, 160), rgb(255, 154, 46), rgb(176, 94, 10), rgb(74, 42, 4)))
    } else if fill == SEQPL() {
        Some((rgb(220, 186, 255), rgb(176, 106, 255), rgb(106, 53, 176), rgb(44, 16, 80)))
    } else {
        None
    };

    if let Some((core, mid, outer, border)) = lit {
        let clip = painter.with_clip_rect(rect);
        // Rim fills the whole pad (corners take the darkest stop).
        clip.rect_filled(rect, r, outer);
        // Radial: mid then core circle centred at 50% / 30%.
        let c = egui::pos2(rect.center().x, rect.top() + rect.height() * 0.30);
        let w = rect.width();
        clip.circle_filled(c, w * 0.52, mid);
        clip.circle_filled(c, w * 0.26, core);
        // Inset top highlight (rgba white .5) + border.
        clip.line_segment(
            [
                egui::pos2(rect.left() + 1.5, rect.top() + 1.0),
                egui::pos2(rect.right() - 1.5, rect.top() + 1.0),
            ],
            egui::Stroke::new(1.0, Color32::from_white_alpha(128)),
        );
        painter.rect_stroke(rect, r, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
        return;
    }

    // Off pads: vertical gradient + tinted border, per fd-skeuo.css.
    let (top, bottom, border) = if fill == CELL_EMPTY_BEAT() {
        (rgb(55, 56, 62), rgb(44, 45, 50), rgb(19, 19, 23))
    } else if fill == CELL_PL_LINK_OFF() {
        (rgb(58, 45, 28), rgb(46, 35, 20), rgb(74, 42, 4))
    } else if fill == CELL_PL_SNAP_OFF() {
        (rgb(58, 33, 28), rgb(46, 24, 20), rgb(74, 15, 8))
    } else if fill == CELL_SEQPL_OFF() {
        (rgb(50, 40, 62), rgb(39, 30, 48), rgb(44, 16, 80))
    } else if fill == FUSION_FILL() {
        (rgb(39, 64, 88), rgb(28, 47, 66), rgb(19, 19, 23))
    } else {
        // Empty off cell (and playhead-current, whose ring is drawn separately).
        (rgb(49, 50, 55), rgb(40, 41, 45), rgb(19, 19, 23))
    };

    // Rounded base in the bottom colour so the corners stay filled, then the
    // gradient inset by 1px, then the recessed-rubber bevels and border.
    painter.rect_filled(rect, r, bottom);
    fill_vgrad(painter, rect.shrink(1.0), top, bottom);
    painter.line_segment(
        [
            egui::pos2(rect.left() + 2.0, rect.top() + 1.0),
            egui::pos2(rect.right() - 2.0, rect.top() + 1.0),
        ],
        egui::Stroke::new(1.0, Color32::from_white_alpha(18)),
    );
    painter.line_segment(
        [
            egui::pos2(rect.left() + 2.0, rect.bottom() - 1.0),
            egui::pos2(rect.right() - 2.0, rect.bottom() - 1.0),
        ],
        egui::Stroke::new(1.0, Color32::from_black_alpha(90)),
    );
    painter.rect_stroke(rect, r, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
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
            CELL_DISABLED(),
            egui::Stroke::new(1.0, Color32::BLACK),
        );
    }
    if selection_start {
        return (FUSION_FILL(), egui::Stroke::new(1.5, BLUE()));
    }

    let empty = if local_step % 4 == 0 {
        CELL_EMPTY_BEAT()
    } else {
        CELL_EMPTY_OFF()
    };

    // Compute the base color as if the cell were not fused.
    let (fill, border) = if sequencer_mode {
        if active && has_seq_plock {
            (SEQPL(), SEQPL())
        } else if has_seq_plock {
            (CELL_SEQPL_OFF(), SEQPL_DIM())
        } else if active {
            (BLUE(), BLUE())
        } else if is_current {
            (CELL_CURRENT(), LINE())
        } else {
            (empty, LINE())
        }
    } else if active && has_sound_plock {
        (PL_LINK(), PL_LINK())
    } else if active {
        (BLUE(), BLUE())
    } else if has_sound_plock {
        if is_snapshot {
            (CELL_PL_SNAP_OFF(), PL_SNAP_DIM())
        } else {
            (CELL_PL_LINK_OFF(), PL_LINK_DIM())
        }
    } else if is_current {
        (CELL_CURRENT(), LINE())
    } else {
        (empty, LINE())
    };

    // Fusion overrides: keep the plock color if the fused cell carries a sound
    // plock or a morph (modulation), so fused cells are not visually muted.
    if is_fusion_start && !is_editing {
        if has_sound_plock || has_seq_plock {
            return (fill, egui::Stroke::new(1.0, BLUE()));
        }
        return (FUSION_FILL(), egui::Stroke::new(1.0, BLUE()));
    }
    if is_fusion_mid && !is_editing {
        return (Color32::TRANSPARENT, egui::Stroke::NONE);
    }

    if is_editing {
        let time = ctx.input(|i| i.time) as f32;
        let pulse = ((time * 4.0).sin() + 1.0) * 0.5;
        // When editing a fused group, pulse every cell of the group using the
        // same blue fusion base so the block flashes as one unit.
        let fusion_fill = if is_fusion_start || is_fusion_mid {
            FUSION_FILL()
        } else {
            fill
        };
        let edit_color = Color32::from_rgba_unmultiplied(
            (BLUE().r() as f32 * pulse + fusion_fill.r() as f32 * (1.0 - pulse)) as u8,
            (BLUE().g() as f32 * pulse + fusion_fill.g() as f32 * (1.0 - pulse)) as u8,
            (BLUE().b() as f32 * pulse + fusion_fill.b() as f32 * (1.0 - pulse)) as u8,
            255,
        );
        return (edit_color, egui::Stroke::new(1.5, BLUE()));
    }

    (fill, egui::Stroke::new(1.0, border))
}

fn draw_mini_value_slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    width: f32,
    fill: Color32,
    tooltip: &str,
) -> egui::Response {
    let style = slider::TrackStyle {
        fill,
        ..slider::TrackStyle::mini()
    };
    let response = slider::draw_track(ui, value, min, max, default, false, width, style);

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
    let response = draw_mini_value_slider(
        ui,
        &mut value,
        min,
        max,
        param.default_plain_value(),
        width,
        fill,
        "",
    );
    if response.changed() {
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
                .fill(P_ACTIVE())
                .stroke(egui::Stroke::new(1.0, LINE2()))
                .corner_radius(RADIUS_CTL)
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 5,
                    bottom: 5,
                })
                .show(ui, |ui| {
                    ui.label(RichText::new(text).font(f_mono_med(10.5)).color(INK()));
                });
        });
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

pub fn preserve_step_active_from_plock_popup(
    pattern_for_ui: &SharedPattern,
    state: &mut EditorUIState,
    instrument: usize,
    step: usize,
    step_was_active: bool,
) {
    if step_was_active && !pattern_for_ui.is_active(step, instrument) {
        set_step_active_for_ui(pattern_for_ui, step, instrument, true);
        state.mark_pattern_dirty();
    }
}

/// Move a single step (with its sound and sequencer plocks) from `source` to
/// `target` within the same slot. Fused cells are left untouched.
fn move_step_with_plocks(
    pattern: &SharedPattern,
    plock: &PlockState,
    params: &DrumFlashParams,
    slot: usize,
    source: usize,
    target: usize,
) {
    if source == target {
        return;
    }

    // Copy step state.
    let active = pattern.is_active(source, slot);
    set_step_active_for_ui(pattern, target, slot, active);
    set_step_active_for_ui(pattern, source, slot, false);

    // Copy sound plock field-by-field so link/snapshot mode is preserved.
    if plock.masks.is_active(slot, source) {
        let field_mask = plock.field_masks.get(slot, source);
        plock.masks.set_active(slot, target, true);
        plock.field_masks.set_raw(slot, target, field_mask);
        for field in 0..crate::plock::FIELD_COUNT {
            plock
                .values
                .set(slot, target, field, plock.values.get(slot, source, field));
        }
        plock.clear(slot, source);
    } else {
        plock.clear(slot, target);
    }

    // Copy sequencer plock.
    let seq = &params.seq_plock_state.state;
    if let Some(seq_params) = seq.get(slot, source) {
        seq.set(slot, target, &seq_params);
    }
    seq.clear(slot, source);
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
    state.mark_pattern_dirty();
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
                .color(BLUE()),
        );
    } else {
        ui.label(RichText::new("Maj for fusion mode").size(11.0).color(INK2()));
    }
}

fn current_field_value_for_fusion(
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    instrument: usize,
    field_index: usize,
    step: usize,
) -> f32 {
    // If the start cell of the fusion has a sound plock for this field, use it
    // as the default value instead of the global lane value.
    let plock = &params.plock_state.state;
    if plock.masks.is_active(instrument, step) {
        let mask = plock.field_masks.get(instrument, step);
        if mask & (1u64 << field_index) != 0 {
            return plock.values.get(instrument, step, field_index);
        }
    }
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
pub fn fusion_morph_state(
    new_fusions: &[crate::sequencer::pattern::FusedGroup],
    fusion_index: usize,
    field_index: usize,
    params: &DrumFlashParams,
    sound_settings: &SoundSettingsState,
    instrument: usize,
    step: usize,
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
            current_field_value_for_fusion(params, sound_settings, instrument, field_index, step),
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
                .fill(PANEL3())
                .stroke(egui::Stroke::new(1.0, LINE2()))
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
                                        .color(INK2()),
                                );
                            } else {
                                ui.label(RichText::new("M: Off").size(10.0).color(INK2()));
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

pub fn copy_page_to_clipboard(
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

pub fn paste_page_from_clipboard(
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

pub fn clear_page_for_ui(
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
pub fn clear_all_fusions(pattern: &SharedPattern) {
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



// ---------------------------------------------------------------------------------------------------------------
// Plock context menu
// ---------------------------------------------------------------------------------------------------------------

