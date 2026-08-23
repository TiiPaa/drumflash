//! P-lock menus: sound plocks, fusion morph, sequencer plocks, popup.

use crate::plock::PlockState;
use crate::sequencer::{
    FusedGroup, SharedPattern,
};
use crate::sound_settings::SoundSettingsState;
use crate::synthesis::VoiceSettings;
use crate::ui::editor_state::*;
use crate::ui::grid::preserve_step_active_from_plock_popup;
use crate::ui::local_param_slider::LocalParamSlider;
use crate::ui::menus::*;
use crate::ui::theme::*;
use crate::DrumFlashParams;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};

fn draw_plock_menu(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    plock: &PlockState,
    sound_settings: &SoundSettingsState,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    step: usize,
    step_was_active: bool,
    state: &mut EditorUIState,
) {
    use crate::plock::FIELD_COUNT;

    #[allow(non_snake_case)] let ACCENT: Color32 = PL_LINK();
    // `instrument` is a SLOT index (plock storage is per slot); registry and
    // special-param lookups go through the voice index of the slot's kind.
    let voice_idx = schema_voice_idx(params, instrument);
    let inst_def = &crate::instrument_registry::INSTRUMENTS[voice_idx];
    let title = format!("Plock {}", inst_def.label);

    preserve_step_active_from_plock_popup(pattern, state, instrument, step, step_was_active);

    // Copy and Paste finish the gesture, so the menu closes afterwards. Set
    // inside the frame closure and applied once it returns: the paste branches
    // hold a borrow on the clipboard, which lives in `state`.
    let mut close_after_action = false;

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
                    .color(INK2()),
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
                        // The gesture is done: the next one happens on another
                        // cell, so keep the grid clear.
                        close_after_action = true;
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
            ui.label(RichText::new("Mode").font(f_sans_med(10.0)).color(INK3()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(mode_text)
                        .font(f_mono_med(10.0))
                        .color(ACCENT),
                );
            });
        });
        ui.add_space(8.0);

        // [184] ph. 4 — the per-parameter rows lived here AND in the Lane Editor,
        // two implementations of the same thing over the same storage. They now
        // live only in the panel, which has a scroll area, the envelope graphs,
        // every one of the 32 specials, and the override markers. This menu keeps
        // what it alone can do: create, copy, paste and clear the p-lock as a
        // whole. Right-clicking a step already aims the panel at it.
        if plock_menu_action_row(ui, "Edit In Panel", ACCENT).clicked() {
            state.sound_edit_target = Some(crate::ui::editor_state::SelectedCell {
                slot: instrument,
                step,
            });
            state.sound_editor_tab = crate::ui::editor_state::SoundEditorTab::Sound;
            state.plock_popup = None;
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
            close_after_action = true;
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
                    close_after_action = true;
                }
            }
        }
        if plock_menu_action_row(ui, "Clear Plock", DANGER()).clicked() {
            plock.clear(instrument, step);
        }
    });
    if close_after_action {
        state.plock_popup = None;
    }
}

/// Fusion group actions (Morphing / Edit Fusion Steps / Delete Fusion),
/// shared by the sound-plock and sequencer-plock popup branches.
fn draw_fusion_group_menu(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    inst: usize,
    idx: usize,
    group: FusedGroup,
    step: usize,
    state: &mut EditorUIState,
) {
    plock_menu_frame(ui, PL_LINK(), |ui| {
        if plock_menu_header(
            ui,
            &format!("Fusion {}-{}", group.start_cell + 1, group.end_cell + 1),
            step,
            PL_LINK(),
        ) {
            state.plock_popup = None;
        }

        let morph_active = group.morph_count > 0;
        let morph_label = if morph_active {
            let morphable =
                crate::instrument_registry::morphable_fields(schema_voice_idx(params, inst));
            let names: Vec<&str> = group.morph_targets[..group.morph_count as usize]
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
        // [184] ph. 4 — the morph is edited in the Lane Editor now: this row just
        // aims the panel at it and closes the popup.
        if plock_menu_action_row(ui, &morph_label, PL_LINK()).clicked() {
            state.sound_edit_target = Some(crate::ui::editor_state::SelectedCell {
                slot: inst,
                step,
            });
            state.fusion_tab = crate::ui::editor_state::FusionTab::End;
            state.sound_editor_tab = crate::ui::editor_state::SoundEditorTab::Sound;
            state.plock_popup = None;
        }
        if plock_menu_action_row(ui, "Edit Fusion Steps", PL_LINK()).clicked() {
            state.fusion_editing = Some((inst, idx));
            state.fusion_edit_steps = group.step_count;
            state.fusion_edit_focus_request = true;
            state.plock_popup = None;
        }
        if plock_menu_action_row(ui, "Delete Fusion", DANGER()).clicked() {
            let mut new_fusions = pattern.load_fusions(inst);
            if idx < new_fusions.len() {
                new_fusions.remove(idx);
                pattern.store_fusions(inst, &new_fusions);
            }
            state.mark_pattern_dirty();
            state.plock_popup = None;
        }
    });
}

pub fn draw_plock_popup(
    ctx: &egui::Context,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    sound_settings: &SoundSettingsState,
    plock: &PlockState,
    state: &mut EditorUIState,
) {
    let popup = match state.plock_popup {
        Some(p) => p,
        None => return,
    };

    let area_id = egui::Id::new("plock_popup");
    let response = egui::Area::new(area_id)
        .kind(egui::UiKind::Menu)
        .order(egui::Order::Foreground)
        .fixed_pos(popup.screen_pos)
        .sense(egui::Sense::click())
        .show(ctx, |ui| {
            // Outer border: draw a slightly larger rounded rect behind the panel.
            let content_response = egui::Frame::NONE
                .fill(P_ACTIVE())
                .corner_radius(RADIUS_PANEL)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_min_width(260.0);
                    ui.set_max_width(350.0);

                    let inst = popup.instrument;
                    let step = popup.step;
                    let fusions = pattern.load_fusions(inst);
                    let fusion_info = fusions.iter().enumerate().find(|(_, g)| {
                        (g.start_cell as usize) <= step && step <= (g.end_cell as usize)
                    });

                    if state.sequencer_mode {
                        if let Some((idx, group)) = fusion_info {
                            // Fused cell: fusion actions (morph/edit/delete)
                            // on top, the seq-plock menu below â€” same as
                            // the sound-plock branch.
                            draw_fusion_group_menu(
                                ui, pattern, params, inst, idx, *group, step,
                                state,
                            );
                            ui.separator();
                            draw_sequencer_plock_menu(
                                ui,
                                pattern,
                                params,
                                setter,
                                inst,
                                step,
                                popup.step_was_active,
                                state,
                                true,
                            );
                        
                        } else {
                            draw_sequencer_plock_menu(
                                ui,
                                pattern,
                                params,
                                setter,
                                inst,
                                step,
                                popup.step_was_active,
                                state,
                                false,
                            );
                        }
                    } else {
                        if let Some((idx, group)) = fusion_info {
                            draw_fusion_group_menu(
                                ui, pattern, params, inst, idx, *group, step,
                                state,
                            );
                            ui.separator();

                            // Also show the source-step plock menu below.
                            draw_plock_menu(
                                ui,
                                pattern,
                                plock,
                                sound_settings,
                                params,
                                setter,
                                inst,
                                step,
                                popup.step_was_active,
                                state,
                            );
                        
                        } else {
                            draw_plock_menu(
                                ui,
                                pattern,
                                plock,
                                sound_settings,
                                params,
                                setter,
                                inst,
                                step,
                                popup.step_was_active,
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
                egui::Stroke::new(1.0, LINE2()),
                egui::StrokeKind::Inside,
            );
            content_response
        })
        .response;

    // Close popup on click outside.
    if response.clicked_elsewhere() {
        state.plock_popup = None;
    }

    // Close popup on click in the popup border/padding (consume the click so it
    // does not pass through to the step cell underneath).
    if state.plock_popup.is_some() && response.clicked() {
        state.plock_popup = None;
    }
}

fn draw_sequencer_plock_menu(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    instrument: usize,
    step: usize,
    step_was_active: bool,
    state: &mut EditorUIState,
    stutter_disabled: bool,
) {
    use crate::plock::{SequencerStepParams, StepCondition};

    #[allow(non_snake_case)] let ACCENT: Color32 = SEQPL();
    // `instrument` is a SLOT index; the label comes from the slot's voice schema.
    let inst_def = &crate::instrument_registry::INSTRUMENTS[schema_voice_idx(params, instrument)];
    let title = format!("Seq Plock {}", inst_def.label);

    preserve_step_active_from_plock_popup(pattern, state, instrument, step, step_was_active);

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
            ui.label(RichText::new("Mode").font(f_sans_med(10.0)).color(INK3()));
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

        // Solo â€” mutes every other lane ONLY while the playhead sits on this
        // cell (its step, or the whole span of a fused cell). Per-cell toggle;
        // remove it by toggling the same cell off. Turning it off clears the
        // whole seq-plock when solo was the only thing set, so a solo-only cell
        // does not linger as an empty (paramless) seq-plock.
        {
            let solo_now = current.solo;
            let solo_response = plock_menu_row(ui, "Solo", ACCENT, solo_now, None, |ui| {
                ui.add(crate::ui::widgets::ToggleSwitch::new(solo_now))
            });
            if solo_response.clicked() {
                if solo_now {
                    seq_plock.set_solo(instrument, step, false);
                    if let Some(p) = seq_plock.get(instrument, step) {
                        let no_other_params = p.probability == 1.0
                            && p.stutter_count == 1
                            && p.condition == StepCondition::Always
                            && p.microtiming_ms == 0.0;
                        if no_other_params {
                            seq_plock.clear(instrument, step);
                        }
                    }
                } else {
                    seq_plock.set_solo(instrument, step, true);
                }
                changed_this_frame = true;
            }
        }
        ui.add_space(4.0);

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
                            .without_value()
                            .reset_value(1.0),
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
                        .color(INK3()),
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
                            .without_value()
                            .reset_value(1.0),
                    )
                },
            );
            if stutter_response.changed() {
                let new_stutter = stutter.round() as u8;
                seq_plock.set_stutter(instrument, step, new_stutter);
                changed_this_frame = true;
            }
        }

        // Microtiming (nudge): shifts the whole cell (stutter/fusion pulses
        // included) by -100..+100 ms around its step boundary.
        {
            let mut nudge = current.microtiming_ms.clamp(-100.0, 100.0);
            let nudge_text = if nudge.abs() < 0.5 {
                "0 ms".to_string()
            } else {
                format!("{:+.0} ms", nudge)
            };
            let nudge_response = plock_menu_row(
                ui,
                "Nudge",
                ACCENT,
                has_seq_plock && current.microtiming_ms != 0.0,
                Some(&nudge_text),
                |ui| {
                    ui.add(
                        LocalParamSlider::new(&mut nudge, -100.0..=100.0)
                            .with_width(86.0)
                            .without_value()
                            .reset_value(0.0),
                    )
                },
            );
            if nudge_response.changed() {
                seq_plock.set_microtiming(instrument, step, nudge);
                changed_this_frame = true;
            }
        }

        // Condition
        ui.add_space(8.0);
        ui.label(RichText::new("Condition").font(f_sans_sb(10.0)).color(INK2()));
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
                    let text_color = if selected { ACCENT } else { INK2() };
                    let fill = if selected { PANEL2() } else { PANEL2() };
                    let stroke_color = if selected { ACCENT } else { LINE2() };
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
                            .corner_radius(RADIUS_CTL),
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
            if plock_menu_action_row(ui, "Clear Seq Plock", DANGER())
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

