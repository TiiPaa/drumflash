//! skeuo_widgets.rs — Flash Drum, dessin des widgets Skeuo pour egui.
//! Stratégie : PADS + petits éléments = textures bakées (png/), le reste = vectoriel
//! (dégradés verticaux approximés par bandes de rects — recette écrite une fois ici).
//! Cible : egui >= 0.29 (+ egui_extras avec feature "image" pour le loader PNG).
//! Sur 0.26–0.28 : `CornerRadius` -> `Rounding`, `Image::corner_radius` -> `rounding`.

#![allow(dead_code)]
use egui::{
    epaint::CornerRadius, Color32, Image, ImageSource, Painter, Pos2, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use crate::skeuo_theme as th;

// ============ Textures bakées (4x — nettes à l'échelle 1) ============
// Init une fois : egui_extras::install_image_loaders(ctx);
pub struct PadTextures;
impl PadTextures {
    pub fn source(state: PadState) -> ImageSource<'static> {
        match state {
            PadState::Off        => egui::include_image!("../png/pad-off.png"),
            PadState::OffBeat    => egui::include_image!("../png/pad-off-beat.png"),
            PadState::Hit        => egui::include_image!("../png/pad-hit.png"),
            PadState::HitLink    => egui::include_image!("../png/pad-hit-link.png"),
            PadState::HitSnap    => egui::include_image!("../png/pad-hit-snap.png"),
            PadState::OffLink    => egui::include_image!("../png/pad-off-link.png"),
            PadState::OffSnap    => egui::include_image!("../png/pad-off-snap.png"),
            PadState::SeqHit     => egui::include_image!("../png/pad-seq-hit.png"),
            PadState::SeqOff     => egui::include_image!("../png/pad-seq-off.png"),
            PadState::FuseStart  => egui::include_image!("../png/pad-fuse-start.png"),
            PadState::FuseMid    => egui::include_image!("../png/pad-fuse-mid.png"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PadState { Off, OffBeat, Hit, HitLink, HitSnap, OffLink, OffSnap, SeqHit, SeqOff, FuseStart, FuseMid }

/// Pad du séquenceur. Texture + overlays (playhead, chiffre de fusion, hors-longueur).
pub fn pad(ui: &mut Ui, state: PadState, playhead: bool, fuse_pulses: Option<u8>, in_range: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(th::PAD_SIZE, Sense::click());
    let tint = if in_range { Color32::WHITE } else { Color32::WHITE.gamma_multiply(0.28) };
    Image::new(PadTextures::source(state))
        .tint(tint)
        .corner_radius(CornerRadius::same(th::R_PAD as u8))
        .paint_at(ui, rect);
    if playhead {
        ui.painter().rect_stroke(rect.shrink(0.75), CornerRadius::same(th::R_PAD as u8),
            Stroke::new(1.5, th::PLAYHEAD_STROKE), StrokeKind::Inside);
    }
    if let Some(n) = fuse_pulses {
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, n.to_string(),
            egui::FontId::monospace(10.0), Color32::WHITE);
    }
    resp
}

// ============ Recette dégradé vertical (le "bricolage" écrit une seule fois) ============
/// Approxime un linear-gradient vertical par N bandes. Suffisant à ces tailles (< 30 pt).
pub fn vgrad(p: &Painter, rect: Rect, stops: &[(f32, Color32)], radius: f32) {
    // fond = couleur du bas, puis bandes du haut vers le bas (les coins restent propres
    // car seul le rect de fond est arrondi ; les bandes intérieures sont insérées de 1 pt).
    let base = stops.last().unwrap().1;
    p.rect_filled(rect, CornerRadius::same(radius as u8), base);
    let inner = rect.shrink(1.0);
    let n = 12;
    for i in 0..n {
        let t0 = i as f32 / n as f32;
        let c = sample_stops(stops, t0);
        let y0 = inner.top() + inner.height() * t0;
        let y1 = inner.top() + inner.height() * (i as f32 + 1.0) / n as f32;
        p.rect_filled(Rect::from_min_max(Pos2::new(inner.left(), y0), Pos2::new(inner.right(), y1)),
            CornerRadius::ZERO, c);
    }
}
fn sample_stops(stops: &[(f32, Color32)], t: f32) -> Color32 {
    let mut prev = stops[0];
    for &s in stops {
        if t <= s.0 {
            let span = (s.0 - prev.0).max(1e-4);
            return prev.1.lerp_to_gamma(s.1, ((t - prev.0) / span).clamp(0.0, 1.0));
        }
        prev = s;
    }
    stops.last().unwrap().1
}

// ============ Keycap (bouton biseauté) ============
#[derive(Clone, Copy, PartialEq)]
pub enum KeycapState { Rest, PressedBlue, PressedAmber }

pub fn keycap(ui: &mut Ui, label: &str, state: KeycapState, min_w: f32) -> Response {
    let galley = ui.painter().layout_no_wrap(label.into(),
        egui::FontId::proportional(th::FS_KEYCAP), Color32::WHITE);
    let w = (galley.rect.width() + 24.0).max(min_w);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, th::H_KEYCAP), Sense::click());
    let p = ui.painter();
    let (stops, border, text): (&[(f32, Color32)], _, _) = match state {
        KeycapState::Rest => (&[(0.0, Color32::from_rgb(74,75,82)), (0.55, Color32::from_rgb(56,57,63)), (1.0, Color32::from_rgb(51,52,57))], th::KEYCAP_BORDER, th::INK_KEYCAP),
        KeycapState::PressedBlue => (&[(0.0, th::BLUE_D), (0.6, Color32::from_rgb(30,95,146)), (1.0, th::BLUE_PRESSED_BOT)], th::BLUE_BORDER, Color32::from_rgb(234,246,255)),
        KeycapState::PressedAmber => (&[(0.0, th::AMBER_TOP), (0.6, Color32::from_rgb(150,86,14)), (1.0, th::AMBER_BOT)], th::AMBER_BORDER, th::AMBER_TEXT),
    };
    vgrad(p, rect, stops, th::R_KEYCAP);
    p.rect_stroke(rect, CornerRadius::same(th::R_KEYCAP as u8), Stroke::new(1.0, border), StrokeKind::Inside);
    // biseau : liseré clair en haut (repos) / assombrissement haut (enfoncé)
    match state {
        KeycapState::Rest => p.rect_filled(
            Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::new(rect.width() - 2.0, 1.0)),
            CornerRadius::ZERO, Color32::from_white_alpha(41)),
        _ => p.rect_filled(
            Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::new(rect.width() - 2.0, 3.0)),
            CornerRadius::ZERO, Color32::from_black_alpha(70)),
    }
    p.galley(rect.center() - galley.rect.size() / 2.0, galley, text);
    resp
}

/// GENERATE — keycap ambre clair dédié.
pub fn generate_button(ui: &mut Ui) -> Response {
    let galley = ui.painter().layout_no_wrap("GENERATE".into(), egui::FontId::proportional(th::FS_KEYCAP), th::GEN_TEXT);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.rect.width() + 32.0, th::H_KEYCAP), Sense::click());
    let p = ui.painter();
    vgrad(p, rect, &[(0.0, th::GEN_TOP), (0.55, th::GEN_MID), (1.0, th::GEN_BOT)], th::R_KEYCAP);
    p.rect_stroke(rect, CornerRadius::same(th::R_KEYCAP as u8), Stroke::new(1.0, th::GEN_BORDER), StrokeKind::Inside);
    p.galley(rect.center() - galley.rect.size() / 2.0, galley, th::GEN_TEXT);
    resp
}

// ============ Slider ============
/// Piste 5 pt (r3, creux) + fill bleu + capuchon strié 12×19 (r3, 3 bandes).
pub fn hslider(ui: &mut Ui, width: f32, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, th::SLIDER_KNOB.y), Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), Vec2::new(width, th::SLIDER_TRACK_H));
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let t = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
            *value = range.start() + t * (range.end() - range.start());
            resp.mark_changed();
        }
    }
    let t = (*value - range.start()) / (range.end() - range.start());
    let p = ui.painter();
    // creux
    vgrad(p, track, &[(0.0, Color32::from_rgb(18,19,23)), (1.0, Color32::from_rgb(28,29,33))], th::R_MICRO);
    // fill
    let fill = Rect::from_min_max(track.min, Pos2::new(track.left() + track.width() * t, track.bottom()));
    vgrad(p, fill, &[(0.0, th::BLUE), (1.0, th::BLUE_D)], th::R_MICRO);
    // capuchon : 3 bandes horizontales (haut 42% / rainure 16% / bas)
    let knob = Rect::from_center_size(Pos2::new(track.left() + track.width() * t, rect.center().y), th::SLIDER_KNOB);
    let k = knob.shrink(1.0);
    p.rect_filled(knob, CornerRadius::same(th::R_MICRO as u8), Color32::from_rgb(16,16,20)); // bord
    p.rect_filled(Rect::from_min_max(k.min, Pos2::new(k.right(), k.top() + k.height() * 0.42)), CornerRadius::same(2), Color32::from_rgb(92,93,101));
    p.rect_filled(Rect::from_min_max(Pos2::new(k.left(), k.top() + k.height() * 0.42), Pos2::new(k.right(), k.top() + k.height() * 0.58)), CornerRadius::ZERO, Color32::from_rgb(46,47,52));
    p.rect_filled(Rect::from_min_max(Pos2::new(k.left(), k.top() + k.height() * 0.58), k.max), CornerRadius::same(2), Color32::from_rgb(74,75,82));
    resp
}

// ============ LED ============
pub fn led(p: &Painter, center: Pos2, on: bool) {
    let r = th::LED_D / 2.0;
    if on {
        // halo discret (2 cercles alpha) + corps + point spéculaire — pas de blur dans egui
        p.circle_filled(center, r + 2.0, Color32::from_rgba_unmultiplied(74, 182, 255, 26));
        p.circle_filled(center, r, Color32::from_rgb(74, 182, 255));
        p.circle_filled(center - Vec2::new(r * 0.25, r * 0.35), r * 0.45, Color32::from_rgba_unmultiplied(196, 236, 255, 200));
    } else {
        p.circle_filled(center, r, Color32::from_rgb(58, 59, 65));
        p.circle_filled(center - Vec2::new(r * 0.2, r * 0.3), r * 0.4, Color32::from_rgb(90, 91, 98));
    }
}

// ============ Écran LCD (cadre ADSR) ============
pub fn lcd_frame(p: &Painter, rect: Rect) {
    vgrad(p, rect, &[(0.0, th::LCD_BG_EDGE), (0.5, th::LCD_BG_MID), (1.0, th::LCD_BG_EDGE)], th::R_PAD);
    p.rect_stroke(rect, CornerRadius::same(th::R_PAD as u8), Stroke::new(1.0, th::LCD_BORDER), StrokeKind::Inside);
    // ombre interne haute (le creux)
    p.rect_filled(Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::new(rect.width() - 2.0, 3.0)),
        CornerRadius::ZERO, Color32::from_black_alpha(90));
}

// ============ Puits / plaque ============
pub fn well(p: &Painter, rect: Rect) {
    vgrad(p, rect, &[(0.0, th::WELL_TOP), (1.0, th::WELL_BOT)], th::R_KEYCAP);
    p.rect_stroke(rect, CornerRadius::same(th::R_KEYCAP as u8), Stroke::new(1.0, Color32::from_rgb(18,18,21)), StrokeKind::Inside);
    p.rect_filled(Rect::from_min_size(rect.min + Vec2::new(1.0, 1.0), Vec2::new(rect.width() - 2.0, 4.0)),
        CornerRadius::ZERO, Color32::from_black_alpha(80)); // inset top shadow
}
