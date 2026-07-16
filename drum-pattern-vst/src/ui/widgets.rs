use crate::ui::theme::*;
use nih_plug_egui::egui::{self, Color32, Response, Sense, StrokeKind, Ui, Vec2, Widget};
use std::hash::Hash;

// ============================================================
// ToggleSwitch — 34×18 r10
// ============================================================
pub struct ToggleSwitch {
    on: bool,
}

impl ToggleSwitch {
    pub fn new(on: bool) -> Self {
        Self { on }
    }
}

impl Widget for ToggleSwitch {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(34.0, 18.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            ui.ctx().request_repaint();
        }

        let on = self.on;
        let painter = ui.painter_at(rect);
        let r = rect.shrink(1.0);

        // Fond
        let bg = if on { blue_glow(64) } else { PANEL2 };
        painter.rect_filled(r, 10.0, bg);

        // Bordure
        let stroke_color = if on { BLUE } else { LINE2 };
        painter.rect_stroke(
            r,
            10.0,
            egui::Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );

        // Pastille
        let knob_radius = 6.0;
        let knob_x = if on {
            r.right() - 5.0 - knob_radius
        } else {
            r.left() + 5.0 + knob_radius
        };
        let knob_center = egui::Pos2::new(knob_x, r.center().y);
        let knob_color = if on { BLUE } else { INK3 };
        painter.circle_filled(knob_center, knob_radius, knob_color);

        response
    }
}

// ============================================================
// ToggleLED — pilule h26 r7, LED Ø7
// ============================================================
pub struct ToggleLED {
    label: String,
    on: bool,
    enabled: bool,
}

impl ToggleLED {
    pub fn new_enabled(label: &str, on: bool, enabled: bool) -> Self {
        Self {
            label: label.to_owned(),
            on,
            enabled,
        }
    }
}

impl Widget for ToggleLED {
    fn ui(self, ui: &mut Ui) -> Response {
        let on = self.on;
        let enabled = self.enabled;
        let font = f_sans_sb(11.0);
        let text_w = ui.fonts(|f| {
            f.layout_no_wrap(self.label.clone(), font.clone(), INK)
                .size()
                .x
        });
        // padding 12 + LED 7 + gap 7 + text + padding 12
        let w = 12.0 + 7.0 + 7.0 + text_w + 12.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(w, CTL_HEIGHT), if enabled { Sense::click() } else { Sense::hover() });

        if enabled && response.clicked() {
            ui.ctx().request_repaint();
        }

        let painter = ui.painter_at(rect);
        let bg = if on && enabled { blue_glow(64) } else { PANEL2 };
        painter.rect_filled(rect, 6.0, bg);
        let stroke_color = if on && enabled { BLUE } else { LINE2 };
        painter.rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );

        // LED Ø7 (+ soft glow when on)
        let led_center = egui::pos2(rect.left() + 12.0 + 3.5, rect.center().y);
        if on && enabled {
            painter.circle_filled(led_center, 5.5, blue_glow(90));
        }
        let led_color = if on && enabled {
            BLUE
        } else if !enabled {
            INK2
        } else {
            FAINT
        };
        painter.circle_filled(led_center, 3.5, led_color);

        let text_color = if !enabled {
            INK2
        } else if on || response.hovered() {
            INK
        } else {
            INK2
        };
        painter.text(
            egui::pos2(rect.left() + 12.0 + 7.0 + 7.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &self.label,
            font,
            text_color,
        );

        response
    }
}

// ============================================================
// Styled select — maquette `.selbox` (h26, r6, mono current)
// ============================================================
pub fn styled_select(
    ui: &mut Ui,
    id_salt: impl Hash,
    selected: usize,
    options: &[&str],
    width: f32,
) -> (Response, Option<usize>) {
    let width = width.max(48.0);
    let selected = selected.min(options.len().saturating_sub(1));
    let current = options.get(selected).copied().unwrap_or("?");
    let popup_id = ui.make_persistent_id(("styled_select", id_salt));
    let popup_open = ui.memory(|mem| mem.is_popup_open(popup_id));

    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(width, CTL_HEIGHT), Sense::click());
    if response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
        ui.ctx().request_repaint();
    }

    let hovered = response.hovered();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, PANEL2);
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, if hovered || popup_open { BLUE } else { LINE2 }),
        StrokeKind::Inside,
    );

    painter.text(
        egui::pos2(rect.left() + 9.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        current,
        f_mono_med(11.0),
        INK,
    );
    // Down-pointing triangle caret
    let caret_size = 5.0;
    let caret_center = egui::pos2(rect.right() - 11.0, rect.center().y + 0.5);
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(
                caret_center.x - caret_size,
                caret_center.y - caret_size * 0.4,
            ),
            egui::pos2(
                caret_center.x + caret_size,
                caret_center.y - caret_size * 0.4,
            ),
            egui::pos2(caret_center.x, caret_center.y + caret_size * 0.9),
        ],
        INK3,
        egui::Stroke::NONE,
    ));

    let mut picked = None;
    if popup_open {
        let mut pos = rect.left_bottom() + Vec2::new(0.0, 4.0);
        if let Some(to_global) = ui.ctx().layer_transform_to_global(ui.layer_id()) {
            pos = to_global * pos;
        }

        let area_response = egui::Area::new(popup_id)
            .kind(egui::UiKind::Popup)
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(P_ACTIVE)
                    .stroke(egui::Stroke::new(1.0, LINE2))
                    .corner_radius(6.0)
                    .inner_margin(0.0)
                    .show(ui, |ui| {
                        ui.set_min_width(width);
                        for (idx, option) in options.iter().enumerate() {
                            let (opt_rect, opt_response) =
                                ui.allocate_exact_size(Vec2::new(width, 24.0), Sense::click());
                            let opt_hovered = opt_response.hovered();
                            if opt_hovered {
                                ui.painter().rect_filled(opt_rect, 0.0, BLUE);
                            } else if idx == selected {
                                ui.painter().rect_filled(opt_rect, 0.0, PANEL2);
                            }
                            ui.painter().text(
                                egui::pos2(opt_rect.left() + 10.0, opt_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                *option,
                                f_sans_med(11.0),
                                if opt_hovered { Color32::WHITE } else { INK2 },
                            );
                            if opt_response.clicked() {
                                picked = Some(idx);
                            }
                        }
                    });
            });

        let close_for_pick = picked.is_some();
        let close_for_escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
        let close_for_outside =
            response.clicked_elsewhere() && area_response.response.clicked_elsewhere();
        if close_for_pick || close_for_escape || close_for_outside {
            ui.memory_mut(|mem| mem.close_popup());
        }
    }

    if picked.is_some() {
        response.mark_changed();
    }

    (response, picked)
}

// ============================================================
// LED segmented control (header "clock source" style)
// One r6 container, per-segment LED dot, blue-glow active fill.
// ============================================================
pub fn led_segmented(ui: &mut Ui, options: &[&str], selected: usize) -> usize {
    let font = f_sans_sb(11.0);
    let h = CTL_HEIGHT;
    let seg_ws: Vec<f32> = options
        .iter()
        .map(|opt| {
            let tw = ui.fonts(|f| {
                f.layout_no_wrap((*opt).to_string(), font.clone(), INK)
                    .size()
                    .x
            });
            // padding 12 + LED 6 + gap 6 + text + padding 12
            12.0 + 6.0 + 6.0 + tw + 12.0
        })
        .collect();
    let total: f32 = seg_ws.iter().sum();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total, h), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, PANEL2);

    let mut result = selected;
    let mut x = rect.left();
    for (i, opt) in options.iter().enumerate() {
        let w = seg_ws[i];
        let seg = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(w, h));
        let is_on = i == selected;
        let resp = ui.interact(
            seg,
            ui.make_persistent_id(("ledseg", *opt, i)),
            Sense::click(),
        );

        if is_on {
            painter.rect_filled(seg.shrink(1.0), 0.0, blue_glow(64));
        }
        if i > 0 {
            painter.line_segment(
                [
                    egui::pos2(seg.left(), rect.top() + 1.0),
                    egui::pos2(seg.left(), rect.bottom() - 1.0),
                ],
                egui::Stroke::new(1.0, LINE2),
            );
        }
        let led_center = egui::pos2(seg.left() + 12.0 + 3.0, seg.center().y);
        if is_on {
            painter.circle_filled(led_center, 5.0, blue_glow(90));
        }
        painter.circle_filled(led_center, 3.0, if is_on { BLUE } else { FAINT });
        let txt_color = if is_on || resp.hovered() { INK } else { INK2 };
        painter.text(
            egui::pos2(seg.left() + 12.0 + 6.0 + 6.0, seg.center().y),
            egui::Align2::LEFT_CENTER,
            *opt,
            font.clone(),
            txt_color,
        );
        if resp.clicked() {
            result = i;
        }
        x += w;
    }
    painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, LINE2), StrokeKind::Inside);
    result
}
