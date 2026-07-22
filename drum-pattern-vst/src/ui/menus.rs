//! Menu chrome: frames, headers and rows shared by the plock, page, settings
//! and add-module popups.

use crate::ui::theme::*;
use crate::ui::widgets::styled_select;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};

pub fn plock_menu_frame(ui: &mut egui::Ui, accent: Color32, content: impl FnOnce(&mut egui::Ui)) {
    // Remove the default context-menu border/shadow so our inner frame is the only chrome.
    ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    ui.visuals_mut().widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);

    let frame = egui::Frame::NONE
        .fill(P_ACTIVE())
        .corner_radius(RADIUS_PANEL)
        .inner_margin(egui::Margin::same(12));
    frame.show(ui, |ui| {
        ui.set_min_width(280.0);
        ui.set_max_width(350.0);
        // Top accent bar
        let bar_rect = ui.available_rect_before_wrap();
        let bar_rect = egui::Rect::from_min_max(
            bar_rect.left_top(),
            egui::pos2(bar_rect.right(), bar_rect.top() + 3.0),
        );
        ui.painter().rect_filled(bar_rect, 0.0, accent);
        ui.add_space(8.0);
        content(ui);
    });
}

pub fn page_menu_frame(ui: &mut egui::Ui, accent: Color32, content: impl FnOnce(&mut egui::Ui)) {
    ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    ui.visuals_mut().widgets.noninteractive.corner_radius = egui::CornerRadius::same(0);

    let frame = egui::Frame::NONE
        .fill(P_ACTIVE())
        .corner_radius(RADIUS_PANEL)
        .inner_margin(egui::Margin::same(10));
    frame.show(ui, |ui| {
        ui.set_min_width(130.0);
        ui.set_max_width(150.0);
        let bar_rect = ui.available_rect_before_wrap();
        let bar_rect = egui::Rect::from_min_max(
            bar_rect.left_top(),
            egui::pos2(bar_rect.right(), bar_rect.top() + 3.0),
        );
        ui.painter().rect_filled(bar_rect, 0.0, accent);
        ui.add_space(8.0);
        content(ui);
    });
}

pub fn plock_menu_header(ui: &mut egui::Ui, title: &str, _step: usize, accent: Color32) -> bool {
    let mut close_clicked = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).font(f_sans_sb(11.0)).color(accent));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button_size = egui::Vec2::new(22.0, 22.0);
            let response = ui.allocate_response(button_size, egui::Sense::click());
            let pressed = response.is_pointer_button_down_on();

            let (fill, text_color) = if pressed {
                (accent, INK())
            } else {
                (Color32::TRANSPARENT, INK3())
            };

            if pressed {
                ui.painter().rect_filled(response.rect, RADIUS_CTL, fill);
            }
            ui.painter().text(
                response.rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                f_sans_med(14.0),
                text_color,
            );

            if response.clicked() {
                close_clicked = true;
            }
        });
    });
    ui.add_space(4.0);
    close_clicked
}

pub fn page_menu_header(ui: &mut egui::Ui, title: &str, accent: Color32) {
    ui.label(RichText::new(title).font(f_sans_sb(11.0)).color(accent));
    ui.add_space(4.0);
}

pub fn plock_menu_row(
    ui: &mut egui::Ui,
    label: &str,
    accent: Color32,
    overridden: bool,
    value_text: Option<&str>,
    content: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.set_height(22.0);
        let label_color = if overridden { accent } else { INK3() };
        ui.label(
            RichText::new(label)
                .font(f_sans_med(10.5))
                .color(label_color),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let response = content(ui);
            if let Some(text) = value_text {
                ui.add_space(8.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(48.0, 22.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label(RichText::new(text).font(f_mono_med(10.0)).color(INK2()));
                    },
                );
            }
            response
        })
        .inner
    })
    .inner
}

pub fn plock_menu_action_row(ui: &mut egui::Ui, label: &str, accent: Color32) -> egui::Response {
    ui.add_sized(
        Vec2::new(ui.available_width(), 26.0),
        egui::Button::new(RichText::new(label).font(f_sans_med(10.5)).color(accent))
            .fill(PANEL2())
            .stroke(egui::Stroke::new(1.0, LINE2()))
            .corner_radius(6.0),
    )
}

pub fn plock_menu_enum_row(
    ui: &mut egui::Ui,
    label: &str,
    accent: Color32,
    overridden: bool,
    current_value: f32,
    options: &[&str],
    id_salt: &str,
) -> (egui::Response, Option<f32>) {
    let current_idx = (current_value as usize).min(options.len().saturating_sub(1));
    let value_text = options[current_idx];
    let mut picked = None;
    let response = plock_menu_row(ui, label, accent, overridden, Some(value_text), |ui| {
        let (resp, p) = styled_select(ui, id_salt, current_idx, options, 120.0);
        if let Some(p) = p {
            picked = Some(p as f32);
        }
        resp
    });
    (response, picked)
}
