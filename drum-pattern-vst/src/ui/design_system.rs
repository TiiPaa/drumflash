use crate::ui::theme::*;
/**
 * Flash Drum — Design System (layered on top of theme.rs)
 *
 * High-level abstractions : typography, spacing helpers, and simple widgets.
 * Low-level tokens live in `theme.rs`.
 */
use nih_plug_egui::egui::{self, Color32, Response, RichText, Stroke, StrokeKind, Ui, Vec2};

// ============================================================
// 1. Typography helpers (RichText factories)
// ============================================================

/// Helpers that return styled `RichText` using the active design tokens.
pub struct Typography;

impl Typography {
    pub fn h1(text: &str) -> RichText {
        egui::RichText::new(text).size(20.0).strong().color(INK)
    }
    pub fn h2(text: &str) -> RichText {
        egui::RichText::new(text).size(14.0).strong().color(INK)
    }
    pub fn h3(text: &str) -> RichText {
        egui::RichText::new(text).size(12.0).strong().color(INK2)
    }
    pub fn body(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).color(INK)
    }
    pub fn body2(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).color(INK2)
    }
    pub fn mono(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).monospace().color(INK2)
    }
    pub fn mono_edit(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).monospace().color(BLUE)
    }
    pub fn mono_faint(text: &str) -> RichText {
        egui::RichText::new(text)
            .size(10.0)
            .monospace()
            .color(FAINT)
    }
    pub fn tag(text: &str) -> RichText {
        egui::RichText::new(text).size(10.0).strong().color(INK2)
    }
    pub fn tag_active(text: &str) -> RichText {
        egui::RichText::new(text).size(10.0).strong().color(INK)
    }
    pub fn btn(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).strong().color(INK)
    }
    pub fn btn_primary(text: &str) -> RichText {
        egui::RichText::new(text)
            .size(11.0)
            .strong()
            .color(Color32::WHITE)
    }
}

// ============================================================
// 2. Spacing helpers
// ============================================================

pub struct Spacing;

impl Spacing {
    pub const GAP_XS: f32 = GAP_TIGHT; // 3.0
    pub const GAP_SM: f32 = GAP_SM; // 4.0
    pub const GAP_MD: f32 = GAP_MD; // 8.0
    pub const GAP_LG: f32 = 12.0;
    pub const GAP_XL: f32 = 16.0;
    pub const GAP_XXL: f32 = 24.0;

    pub const RADIUS_SM: f32 = 4.0;
    pub const RADIUS_MD: f32 = RADIUS_PILL; // 7.0
    pub const RADIUS_PANEL: f32 = RADIUS_PANEL; // 9.0
    pub const RADIUS_LG: f32 = 12.0;

    pub const ROW_SM: f32 = 18.0;
    pub const ROW_MD: f32 = 22.0;
    pub const ROW_LG: f32 = CTL_HEIGHT; // 26.0
    pub const ROW_XL: f32 = 30.0;
}

// ============================================================
// 3. Widget helpers
// ============================================================

/// Frame style for a card / panel.
pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(RADIUS_PANEL)
        .inner_margin(GAP_LG)
}

/// Vertical divider line.
pub fn vertical_bar(ui: &mut Ui, height: f32) {
    let rect = ui.available_rect_before_wrap();
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x, rect.min.y + (rect.height() - height) / 2.0),
        Vec2::new(1.0, height),
    );
    ui.painter().rect_filled(bar_rect, 0.0, LINE);
}

/// Standard button (non-primary).
pub fn button(ui: &mut Ui, text: &str) -> Response {
    let btn = egui::Button::new(Typography::btn(text))
        .fill(PANEL2)
        .stroke(Stroke::new(1.0, LINE2))
        .corner_radius(RADIUS_PILL)
        .min_size(Vec2::new(0.0, CTL_HEIGHT));
    ui.add(btn)
}

/// Primary accent button.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let btn = egui::Button::new(Typography::btn_primary(text))
        .fill(BLUE)
        .stroke(Stroke::new(1.0, BLUE))
        .corner_radius(RADIUS_PILL)
        .min_size(Vec2::new(0.0, CTL_HEIGHT));
    ui.add(btn)
}

/// Tag / chip.
pub fn tag(ui: &mut Ui, text: &str, active: bool) -> Response {
    let (fill, stroke_color, text_color) = if active {
        (BLUE, BLUE, Color32::WHITE)
    } else {
        (PANEL2, LINE2, INK2)
    };
    let btn = egui::Button::new(
        egui::RichText::new(text)
            .size(10.0)
            .strong()
            .color(text_color),
    )
    .fill(fill)
    .stroke(Stroke::new(1.0, stroke_color))
    .corner_radius(RADIUS_CTL)
    .min_size(Vec2::new(30.0, TAG_SIZE));
    ui.add(btn)
}

/// Panel header with optional subtitle.
pub fn panel_header(ui: &mut Ui, title: &str, sub: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(Typography::h3(title));
        if let Some(s) = sub {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(Typography::mono_faint(s));
            });
        }
    });
    ui.separator();
}
