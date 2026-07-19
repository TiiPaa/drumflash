use nih_plug_egui::egui::{Color32, FontFamily, FontId};

// Surfaces
pub const BG: Color32 = Color32::from_rgb(10, 10, 15);
pub const PANEL: Color32 = Color32::from_rgb(20, 20, 25);
pub const PANEL2: Color32 = Color32::from_rgb(28, 28, 36);
pub const P_HOVER: Color32 = Color32::from_rgb(36, 36, 48);
pub const P_ACTIVE: Color32 = Color32::from_rgb(42, 42, 56);
pub const LINE: Color32 = Color32::from_rgb(42, 42, 53);
pub const LINE2: Color32 = Color32::from_rgb(58, 58, 72);

// Accents
pub const BLUE: Color32 = Color32::from_rgb(74, 158, 255);
pub const GREEN: Color32 = Color32::from_rgb(74, 222, 128);
pub const RED: Color32 = Color32::from_rgb(248, 113, 113);
pub const AMBER: Color32 = Color32::from_rgb(251, 191, 36);

// P-locks (cœur produit)
pub const PL_LINK: Color32 = Color32::from_rgb(255, 140, 0);
pub const PL_LINK_DIM: Color32 = Color32::from_rgb(180, 100, 0);
pub const PL_SNAP_DIM: Color32 = Color32::from_rgb(160, 30, 30);
pub const SEQPL: Color32 = Color32::from_rgb(168, 85, 247);
pub const SEQPL_DIM: Color32 = Color32::from_rgb(120, 60, 180);

// Texte
pub const INK: Color32 = Color32::from_rgb(232, 232, 240);
pub const INK2: Color32 = Color32::from_rgb(156, 163, 175);
pub const INK3: Color32 = Color32::from_rgb(107, 114, 128);
pub const FAINT: Color32 = Color32::from_rgb(75, 85, 99);

// Helpers
// ============================================================
#[inline]
pub fn blue_glow(alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(74, 158, 255, alpha)
}

#[inline]
pub fn white_a(alpha: u8) -> Color32 {
    Color32::from_white_alpha(alpha)
}

#[inline]
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

// ============================================================
// Primitives (rayons, gaps, strokes)
pub const RADIUS_CTL: f32 = 5.0;
pub const RADIUS_PANEL: f32 = 9.0;
pub const GAP_TIGHT: f32 = 3.0;

// Hauteurs / tailles standard
pub const CTL_HEIGHT: f32 = 26.0;
pub const HEADER_H: f32 = 44.0;
pub const LANE_H: f32 = 24.0;
pub const STEP_H: f32 = 21.0;
pub const TAG_SIZE: f32 = 21.0;

// ============================================================
// Font helpers — weighted IBM Plex families (registered in install_egui_fonts).
// Convention: "chiffre = mono, mot = sans" → mono_* for numbers/values/codes.
// ============================================================
#[allow(dead_code)]
#[inline]
pub fn f_sans(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
#[allow(dead_code)]
#[inline]
pub fn f_sans_med(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("sans_med".into()))
}
#[allow(dead_code)]
#[inline]
pub fn f_sans_sb(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("sans_sb".into()))
}
#[allow(dead_code)]
#[inline]
pub fn f_sans_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("sans_bold".into()))
}
#[allow(dead_code)]
#[inline]
pub fn f_mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}
#[allow(dead_code)]
#[inline]
pub fn f_mono_med(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono_med".into()))
}
#[allow(dead_code)]
#[inline]
pub fn f_mono_sb(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono_sb".into()))
}
