//! Header bar: brand, master/swing sliders, groove select, source + toggles.

use crate::ui::controls::{enum_combo, toggle_led_param};
use crate::ui::editor_state::EditorUIState;
use crate::ui::theme::*;
use crate::ui::widgets::led_segmented;
use crate::{DrumFlashParams, BUILD_ID};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use std::sync::{
    atomic::{AtomicBool, AtomicU32},
    Arc,
};

pub fn header_param_slider<P: Param>(
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
            INK2(),
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
            INK(),
        );
        (val_rect.left() - 8.0).max(track_left + 12.0)
    } else {
        rect.right()
    };
    // Reserve the Ø11 knob radius at both ends so it isn't clipped at the extremes.
    let knob_r = 6.0;
    let track_left = track_left.max(rect.left() + knob_r);
    let track_right = track_right
        .min(rect.right() - knob_r)
        .max(track_left + 12.0);
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
    if resp.double_clicked() {
        frac = param.default_normalized_value();
        setter.begin_set_parameter(param);
        setter.set_parameter_normalized(param, frac);
        setter.end_set_parameter(param);
    }

    // All slider visuals live in one place: `skeuo::slider_track` (Len = with cap).
    crate::ui::skeuo::slider_track(ui, track, frac, BLUE(), true);
}

/// A 1px vertical separator (height 22) with 14pt horizontal padding on each side.
fn header_vbar(ui: &mut egui::Ui) {
    ui.add_space(14.0);
    let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 22.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, LINE());
    ui.add_space(14.0);
}

pub fn draw_header_bar(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    state: &mut EditorUIState,
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

    // Skeuo metal header: vertical gradient + top highlight + bottom border.
    let painter = ui.painter_at(rect);
    crate::ui::widgets::vgrad(
        &painter,
        rect,
        &[(0.0, HEADER_TOP), (0.6, HEADER_MID), (1.0, HEADER_BOT)],
        0.0,
    );
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        egui::Stroke::new(1.0, Color32::from_white_alpha(30)),
    );
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0, LINE()),
    );

    // Contenu avec padding horizontal
    let content_rect = rect.shrink2(egui::Vec2::new(14.0, 0.0));
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(content_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
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
                        .color(INK3()),
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
                ui.label(RichText::new("Seq").font(f_sans_sb(10.5)).color(INK3()));
                ui.add_space(8.0);
                let internal = params.use_internal_sequencer.value();
                let sel =
                    led_segmented(ui, &["Internal", "Ext MIDI"], if internal { 0 } else { 1 });
                let want_internal = sel == 0;
                if want_internal != internal {
                    setter.begin_set_parameter(&params.use_internal_sequencer);
                    setter.set_parameter(&params.use_internal_sequencer, want_internal);
                    setter.end_set_parameter(&params.use_internal_sequencer);
                    // MIDI pattern switching only makes sense in internal sequencer mode.
                    if !want_internal && params.midi_pattern_switch.value() {
                        setter.begin_set_parameter(&params.midi_pattern_switch);
                        setter.set_parameter(&params.midi_pattern_switch, false);
                        setter.end_set_parameter(&params.midi_pattern_switch);
                    }
                }

                header_vbar(ui);

                // Toggles (LED pills)
                toggle_led_param(ui, setter, &params.hihat_chokes_oh, "Choke", true);
                ui.add_space(6.0);
                toggle_led_param(ui, setter, &params.auto_edit, "Auto-Edit", true);
                ui.add_space(6.0);
                let midi_pat_enabled = params.use_internal_sequencer.value();
                toggle_led_param(
                    ui,
                    setter,
                    &params.midi_pattern_switch,
                    "MIDI Pat",
                    midi_pat_enabled,
                );

                // Push the settings button to the right edge of the header.
                ui.add_space((ui.available_width() - 60.0).max(0.0));
                if ui
                    .button(RichText::new("Settings").font(f_sans_med(10.5)).color(INK3()))
                    .clicked()
                {
                    state.settings_open = true;
                }
            });
        },
    );
}
