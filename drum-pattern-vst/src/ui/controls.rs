//! Small param-bound controls and keyboard helpers shared across panels.

use crate::ui::theme::*;
use crate::ui::widgets::{styled_select, ToggleLED};
use crate::DrumFlashParams;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};

pub fn set_bool_param_if_changed(setter: &ParamSetter, param: &BoolParam, value: bool) {
    if param.value() != value {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

pub fn set_float_param_if_changed(setter: &ParamSetter, param: &FloatParam, value: f32) {
    if (param.value() - value).abs() > f32::EPSILON {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

pub fn set_int_param_if_changed(setter: &ParamSetter, param: &IntParam, value: i32) {
    if param.value() != value {
        setter.begin_set_parameter(param);
        setter.set_parameter(param, value);
        setter.end_set_parameter(param);
    }
}

pub fn draw_track_length_control(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    length_param: &IntParam,
    instrument: usize,
    master_length: usize,
) -> bool {
    let locked = params.lane_length_locks.is_locked(instrument);
    let raw = length_param.value() as usize;
    let mut length_value = if locked {
        raw as i32
    } else {
        master_length as i32
    };

    let response = ui.add_sized(
        Vec2::new(35.0, 20.0),
        egui::DragValue::new(&mut length_value)
            .speed(1.0)
            .range(1..=64),
    );
    let changed = response.changed();
    let response = response.on_hover_text(if locked {
        "Locked lane length. Right-click to follow pattern length."
    } else {
        "Follows pattern length. Drag to lock this lane."
    });
    let interacted =
        changed || response.clicked() || response.dragged() || response.secondary_clicked();

    response.context_menu(|ui| {
        if locked {
            if ui.button("Follow pattern length").clicked() {
                params.lane_length_locks.set_locked(instrument, false);
                setter.set_parameter(length_param, master_length as i32);
                ui.close_menu();
            }
        } else {
            ui.label("Already follows pattern length");
        }
    });

    if changed {
        params.lane_length_locks.set_locked(instrument, true);
        setter.set_parameter(length_param, length_value.clamp(1, 64));
    }

    interacted
}

pub fn fusion_modifier_pressed(ui: &egui::Ui) -> bool {
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

pub fn toggle_led_param(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &BoolParam,
    label: &str,
    enabled: bool,
) {
    let value = param.value();
    if ui
        .add(ToggleLED::new_enabled(label, value, enabled))
        .clicked()
    {
        let new_value = !value;
        setter.begin_set_parameter(param);
        setter.set_parameter(param, new_value);
        setter.end_set_parameter(param);
    }
}

pub fn enum_combo<E: nih_plug::prelude::Enum + PartialEq + 'static>(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &EnumParam<E>,
    label: &str,
) {
    let current = param.value();
    let current_idx = current.to_index();
    let variants = E::variants();
    if !label.is_empty() {
        ui.label(RichText::new(label).font(f_sans_sb(10.5)).color(INK3()));
    }
    if let (_, Some(i)) = styled_select(ui, ("enum_combo", label), current_idx, variants, 116.0) {
        if i != current_idx {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, E::from_index(i));
            setter.end_set_parameter(param);
        }
    }
}

pub fn enum_combo_compact<E: nih_plug::prelude::Enum + PartialEq + 'static>(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    param: &EnumParam<E>,
    id: &'static str,
    width: f32,
) {
    let current = param.value();
    let current_idx = current.to_index();
    let variants = E::variants();
    if let (_, Some(i)) = styled_select(ui, id, current_idx, variants, width) {
        if i != current_idx {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, E::from_index(i));
            setter.end_set_parameter(param);
        }
    }
}

pub fn compact_chip(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    compact_chip_colored(ui, label, active, BLUE())
}

pub fn compact_chip_colored(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    accent: Color32,
) -> egui::Response {
    let text_color = if active { Color32::WHITE } else { INK2() };
    let fill = if active { accent } else { PANEL2() };
    let stroke = if active { accent } else { LINE2() };
    ui.add(
        egui::Button::new(RichText::new(label).size(10.5).color(text_color))
            .min_size(Vec2::new(42.0, CTL_HEIGHT))
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(6.0),
    )
}

pub fn genrow_label(ui: &mut egui::Ui, label: &str, min_w: f32) {
    ui.add_sized(
        Vec2::new(min_w, CTL_HEIGHT),
        egui::Label::new(RichText::new(label).font(f_mono_med(9.5)).color(INK3())),
    );
}

pub fn chip_button(
    ui: &mut egui::Ui,
    label: &str,
    accent: bool,
    color: Color32,
    sense: egui::Sense,
) -> egui::Response {
    let text_color = if accent { color } else { INK2() };
    let stroke = if accent { color } else { LINE2() };
    let fill = if accent {
        Color32::from_rgba_premultiplied(
            ((color.r() as f32) * 0.12) as u8,
            ((color.g() as f32) * 0.12) as u8,
            ((color.b() as f32) * 0.12) as u8,
            255,
        )
    } else {
        PANEL2()
    };
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(10.5)
                .color(text_color)
                .font(f_sans_sb(11.0)),
        )
        .min_size(Vec2::new(0.0, CTL_HEIGHT))
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(6.0)
        .sense(sense),
    )
}

pub fn algo_combo(ui: &mut egui::Ui, setter: &ParamSetter, param: &IntParam, algo_names: &[&str]) {
    let current = param.value() as usize;
    let current_clamped = current.min(algo_names.len().saturating_sub(1));
    if let (_, Some(i)) = styled_select(ui, "algo_combo", current_clamped, algo_names, 146.0) {
        if i != current_clamped {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, i as i32);
            setter.end_set_parameter(param);
        }
    }
}

pub fn p_lock_mode_segmented(ui: &mut egui::Ui, selected: usize) -> usize {
    text_segmented(
        ui,
        "plock_mode",
        &[("Sound", PL_LINK()), ("Sequencer", SEQPL())],
        selected,
    )
}

pub fn generator_song_segmented(ui: &mut egui::Ui, selected: usize) -> usize {
    text_segmented(
        ui,
        "gen_song_mode",
        &[("Generator", BLUE()), ("Song", PL_LINK())],
        selected,
    )
}

fn text_segmented(
    ui: &mut egui::Ui,
    id_salt: &'static str,
    options: &[(&str, Color32)],
    selected: usize,
) -> usize {
    let font = f_sans_sb(10.5);
    let padding = 24.0; // 12 left + 12 right
    let mut widths = Vec::with_capacity(options.len());
    for (label, _) in options {
        let tw = ui.fonts(|f| {
            f.layout_no_wrap((*label).to_string(), font.clone(), INK())
                .size()
                .x
        });
        widths.push((tw + padding).max(56.0));
    }
    let total_w: f32 = widths.iter().sum();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, CTL_HEIGHT), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, PANEL2());

    let mut result = selected.min(options.len().saturating_sub(1));
    let mut x = rect.left();
    for (idx, (label, accent)) in options.iter().enumerate() {
        let seg = egui::Rect::from_min_size(
            egui::pos2(x, rect.top()),
            Vec2::new(widths[idx], CTL_HEIGHT),
        );
        let active = idx == result;
        let response = ui.interact(
            seg,
            ui.make_persistent_id((id_salt, idx)),
            egui::Sense::click(),
        );
        if active {
            painter.rect_filled(seg.shrink(1.0), 5.0, *accent);
        } else if response.hovered() {
            painter.rect_filled(seg.shrink(1.0), 5.0, P_HOVER());
            painter.rect_stroke(
                seg.shrink(1.0),
                5.0,
                egui::Stroke::new(1.0, *accent),
                egui::StrokeKind::Inside,
            );
        }
        if idx > 0 {
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
            font.clone(),
            if active {
                Color32::WHITE
            } else if response.hovered() {
                INK()
            } else {
                INK2()
            },
        );
        if response.clicked() {
            result = idx;
            ui.ctx().request_repaint();
        }
        x += widths[idx];
    }

    // Draw the outer frame last so active/hover fills cannot visually eat it.
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, LINE2()),
        egui::StrokeKind::Inside,
    );

    result
}