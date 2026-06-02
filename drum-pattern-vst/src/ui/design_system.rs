/**
 * Flash Drum — Design System (egui implementation)
 *
 * Tokens visuels et widgets réutilisables.
 * Source de vérité : docs/design/DESIGN-SYSTEM.md
 */

use nih_plug_egui::egui::{self, Color32, RichText, Stroke, Vec2, Ui, Response};

// ============================================================
// 1. Palette — Couleurs
// ============================================================

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub panel2: Color32,
    pub panel_hover: Color32,
    pub panel_active: Color32,
    pub line: Color32,
    pub line2: Color32,
    pub divider: Color32,
    pub blue: Color32,
    pub blue_dim: Color32,
    pub blue_glow: Color32,
    pub green: Color32,
    pub red: Color32,
    pub amber: Color32,
    pub ink: Color32,
    pub ink2: Color32,
    pub ink3: Color32,
    pub ink_faint: Color32,
    pub ink_blue: Color32,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            bg: Color32::from_rgb(10, 10, 15),
            panel: Color32::from_rgb(20, 20, 25),
            panel2: Color32::from_rgb(28, 28, 36),
            panel_hover: Color32::from_rgb(36, 36, 48),
            panel_active: Color32::from_rgb(42, 42, 56),
            line: Color32::from_rgb(42, 42, 53),
            line2: Color32::from_rgb(58, 58, 72),
            divider: Color32::from_rgb(31, 31, 40),
            blue: Color32::from_rgb(74, 158, 255),
            blue_dim: Color32::from_rgba_premultiplied(74, 158, 255, 128),
            blue_glow: Color32::from_rgba_premultiplied(74, 158, 255, 64),
            green: Color32::from_rgb(74, 222, 128),
            red: Color32::from_rgb(248, 113, 113),
            amber: Color32::from_rgb(251, 191, 36),
            ink: Color32::from_rgb(232, 232, 240),
            ink2: Color32::from_rgb(156, 163, 175),
            ink3: Color32::from_rgb(107, 114, 128),
            ink_faint: Color32::from_rgb(75, 85, 99),
            ink_blue: Color32::from_rgb(74, 158, 255),
        }
    }
}

impl Palette {
    pub fn white_a(alpha: u8) -> Color32 {
        Color32::from_rgba_premultiplied(255, 255, 255, alpha)
    }
}

// ============================================================
// 2. Typographie (helpers RichText)
// ============================================================

#[derive(Clone, Copy, Debug)]
pub struct Typography;

impl Typography {
    pub fn h1(text: &str) -> RichText {
        egui::RichText::new(text).size(20.0).strong().color(Palette::default().ink)
    }
    pub fn h2(text: &str) -> RichText {
        egui::RichText::new(text).size(14.0).strong().color(Palette::default().ink)
    }
    pub fn h3(text: &str) -> RichText {
        egui::RichText::new(text).size(12.0).strong().color(Palette::default().ink2)
    }
    pub fn body(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).color(Palette::default().ink)
    }
    pub fn body2(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).color(Palette::default().ink2)
    }
    pub fn mono(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).monospace().color(Palette::default().ink2)
    }
    pub fn mono_edit(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).monospace().color(Palette::default().ink_blue)
    }
    pub fn mono_faint(text: &str) -> RichText {
        egui::RichText::new(text).size(10.0).monospace().color(Palette::default().ink_faint)
    }
    pub fn tag(text: &str) -> RichText {
        egui::RichText::new(text).size(10.0).strong().color(Palette::default().ink2)
    }
    pub fn tag_active(text: &str) -> RichText {
        egui::RichText::new(text).size(10.0).strong().color(Palette::default().ink)
    }
    pub fn btn(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).strong().color(Palette::default().ink)
    }
    pub fn btn_primary(text: &str) -> RichText {
        egui::RichText::new(text).size(11.0).strong().color(Color32::WHITE)
    }
}

// ============================================================
// 3. Espacement & formes
// ============================================================

#[derive(Clone, Copy, Debug)]
pub struct Spacing;

impl Spacing {
    pub const GAP_XS: f32 = 2.0;
    pub const GAP_SM: f32 = 4.0;
    pub const GAP_MD: f32 = 8.0;
    pub const GAP_LG: f32 = 12.0;
    pub const GAP_XL: f32 = 16.0;
    pub const GAP_XXL: f32 = 24.0;

    pub const RADIUS_SM: f32 = 4.0;
    pub const RADIUS_MD: f32 = 7.0;
    pub const RADIUS_PANEL: f32 = 10.0;
    pub const RADIUS_LG: f32 = 12.0;

    pub const ROW_SM: f32 = 18.0;
    pub const ROW_MD: f32 = 22.0;
    pub const ROW_LG: f32 = 26.0;
    pub const ROW_XL: f32 = 30.0;
}

// ============================================================
// 4. Widgets
// ============================================================

/// Style d'un panneau (carte)
pub fn panel_frame() -> egui::Frame {
    let p = Palette::default();
    egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(Spacing::RADIUS_PANEL)
        .inner_margin(Spacing::GAP_LG)
}

/// Barre de séparation verticale
pub fn vertical_bar(ui: &mut Ui, height: f32) {
    let p = Palette::default();
    let rect = ui.available_rect_before_wrap();
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x, rect.min.y + (rect.height() - height) / 2.0),
        Vec2::new(1.0, height),
    );
    ui.painter().rect_filled(bar_rect, 0.0, p.line);
}

/// Bouton standard (non primaire)
pub fn button(ui: &mut Ui, text: &str) -> Response {
    let p = Palette::default();
    let btn = egui::Button::new(Typography::btn(text))
        .fill(p.panel2)
        .stroke(Stroke::new(1.0, p.line2))
        .corner_radius(Spacing::RADIUS_MD)
        .min_size(Vec2::new(0.0, Spacing::ROW_LG));
    ui.add(btn)
}

/// Bouton primaire (accent)
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let p = Palette::default();
    let btn = egui::Button::new(Typography::btn_primary(text))
        .fill(p.blue)
        .stroke(Stroke::new(1.0, p.blue))
        .corner_radius(Spacing::RADIUS_MD)
        .min_size(Vec2::new(0.0, Spacing::ROW_LG));
    ui.add(btn)
}

/// Tag / chip
pub fn tag(ui: &mut Ui, text: &str, active: bool) -> Response {
    let p = Palette::default();
    let (fill, stroke_color, text_color) = if active {
        (p.blue, p.blue, Color32::WHITE)
    } else {
        (p.panel2, p.line2, p.ink2)
    };
    let btn = egui::Button::new(
        egui::RichText::new(text).size(10.0).strong().color(text_color)
    )
    .fill(fill)
    .stroke(Stroke::new(1.0, stroke_color))
    .corner_radius(Spacing::RADIUS_SM)
    .min_size(Vec2::new(30.0, 18.0));
    ui.add(btn)
}

/// Toggle LED
pub fn toggle_led(ui: &mut Ui, label: &str, on: &mut bool) -> Response {
    let p = Palette::default();
    ui.horizontal(|ui| {
        let led_color = if *on { p.blue } else { p.line };
        let led_size = 8.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(led_size, led_size), egui::Sense::click());
        if response.clicked() {
            *on = !*on;
        }
        ui.painter().circle_filled(rect.center(), led_size / 2.0, led_color);
        ui.label(Typography::body2(label));
        response
    }).inner
}

/// En-tête de panneau
pub fn panel_header(ui: &mut Ui, title: &str, sub: Option<&str>) {
    let p = Palette::default();
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
