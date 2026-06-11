use crate::ui::theme::*;
use nih_plug_egui::egui::{self, Color32, Response, Sense, StrokeKind, Ui, Vec2, Widget};

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
}

impl ToggleLED {
    pub fn new(label: &str, on: bool) -> Self {
        Self {
            label: label.to_owned(),
            on,
        }
    }
}

impl Widget for ToggleLED {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(72.0, CTL_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            ui.ctx().request_repaint();
        }

        let on = self.on;
        let painter = ui.painter_at(rect);
        let r = rect.shrink(1.0);

        // Fond
        let bg = if on { blue_glow(32) } else { PANEL2 };
        painter.rect_filled(r, RADIUS_PILL, bg);

        // Bordure
        let stroke_color = if on { BLUE } else { LINE2 };
        painter.rect_stroke(
            r,
            RADIUS_PILL,
            egui::Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );

        // LED Ø7
        let led_radius = 3.5;
        let led_center = egui::Pos2::new(r.left() + 10.0 + led_radius, r.center().y);
        let led_color = if on { BLUE } else { FAINT };
        painter.circle_filled(led_center, led_radius, led_color);

        // Glow quand actif
        if on {
            painter.circle_filled(led_center, led_radius + 2.0, blue_glow(40));
        }

        // Label
        let text_color = if on { INK } else { INK2 };
        painter.text(
            egui::Pos2::new(r.left() + 20.0, r.center().y),
            egui::Align2::LEFT_CENTER,
            &self.label,
            egui::FontId::proportional(10.5),
            text_color,
        );

        response
    }
}

// ============================================================
// StyledButton — bouton coordonné (h26, r6, bordure 1px LINE2)
// ============================================================
pub struct StyledButton {
    label: String,
    active: bool,
}

impl StyledButton {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_owned(),
            active: false,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl Widget for StyledButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = Vec2::new(64.0, CTL_HEIGHT);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        let hovered = response.hovered();
        let clicked = response.clicked();
        let painter = ui.painter_at(rect);
        let r = rect.shrink(1.0);

        // Fond
        let bg = if self.active {
            BLUE
        } else if hovered {
            P_HOVER
        } else {
            PANEL2
        };
        painter.rect_filled(r, RADIUS_CTL, bg);

        // Bordure
        let stroke_color = if self.active || hovered { BLUE } else { LINE2 };
        painter.rect_stroke(
            r,
            RADIUS_CTL,
            egui::Stroke::new(1.0, stroke_color),
            StrokeKind::Inside,
        );

        // Texte
        let text_color = if self.active { Color32::WHITE } else { INK2 };
        painter.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            &self.label,
            egui::FontId::proportional(11.0),
            text_color,
        );

        response
    }
}

// ============================================================
// SegmentedControl — toggle groupé (ex: Sound/Sequencer)
// ============================================================
pub struct SegmentedControl {
    options: Vec<String>,
    selected: usize,
}

impl SegmentedControl {
    pub fn new(options: &[&str], selected: usize) -> Self {
        Self {
            options: options.iter().map(|s| s.to_string()).collect(),
            selected,
        }
    }
}

/// Segmented control using real egui buttons (reliable interaction).
/// Returns the selected index; if unchanged, no click occurred.
pub fn segmented_control(ui: &mut Ui, options: &[&str], selected: usize) -> (Response, usize) {
    let n = options.len();
    let total_w = 140.0f32;
    let btn_w = total_w / n as f32;
    let mut new_selected = selected;
    let mut any_clicked = false;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0; // no gap between segments

        for (i, label) in options.iter().enumerate() {
            let is_selected = i == selected;

            let bg = if is_selected {
                if *label == "Sequencer" {
                    SEQPL
                } else {
                    PL_LINK
                }
            } else {
                PANEL2
            };
            let stroke_color = if is_selected {
                if *label == "Sequencer" {
                    SEQPL
                } else {
                    PL_LINK
                }
            } else {
                LINE2
            };
            let text_color = if is_selected { Color32::WHITE } else { INK2 };

            let btn = egui::Button::new(egui::RichText::new(*label).size(10.5).color(text_color))
                .min_size(Vec2::new(btn_w, CTL_HEIGHT))
                .fill(bg)
                .stroke(egui::Stroke::new(1.0, stroke_color))
                .corner_radius(0.0);

            let response = ui.add(btn);
            if response.clicked() {
                new_selected = i;
                any_clicked = true;
            }
        }
    });

    let response = ui.interact(ui.min_rect(), ui.id(), Sense::click());

    (response, if any_clicked { new_selected } else { selected })
}
