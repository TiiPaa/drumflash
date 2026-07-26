//! # Skeuo UI elements — the single home for every graphical element's look.
//!
//! Each function here renders exactly ONE UI element (keycap, pad, slider track,
//! recessed well, …). **Every call site in the UI goes through these functions**,
//! so changing an element's appearance is a one-place edit that applies
//! everywhere — no more hunting the same widget across `header.rs`, `slider.rs`,
//! `grid.rs`, etc.
//!
//! ## Vector *or* bitmap — same entry point
//! Every element takes a `ui: &egui::Ui` and a `rect`. The bodies below draw with
//! vector shapes (`ui.painter()` + the primitives in [`crate::ui::widgets`]), but
//! you can swap ANY element to a **baked bitmap** without touching a single call
//! site: replace that one function's body with, e.g.
//!
//! ```ignore
//! egui::Image::new(egui::include_image!("../../assets/keycaps/keycap-rest.png"))
//!     .paint_at(ui, rect);            // (3-slice for variable widths if needed)
//! ```
//!
//! The PNG loader (`egui_extras::install_image_loaders`) is installed once at
//! editor startup, so `include_image!` works anywhere here.
//!
//! ## Where things live
//! - **This file**: the elements (what a thing *is*): `keycap`, `pad`,
//!   `slider_track`, `well_recess`, …
//! - **`widgets.rs`**: low-level primitives (what a thing is *made of*): smooth
//!   gradients, soft shadows, radial fills, fade lines. Reused across elements.
//! - **`theme.rs`**: the palette and geometry constants.

use crate::ui::theme::*;
use crate::ui::widgets::{
    fade_line, grad3, inner_side_shadows, inner_top_shadow, soft_shadow, KeycapState,
};
use nih_plug_egui::egui::{self, Color32};

#[inline]
const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

// ============================================================
// Keycap — every button/tab/page/slot/segment/dropdown background.
// (rest grey / pressed blue / pressed amber). r5, soft shadow + smooth 3-stop
// gradient + dark border + subtle relief liseré.
// ============================================================
pub fn keycap(ui: &egui::Ui, rect: egui::Rect, state: KeycapState) {
    let p = ui.painter();
    let radius = 5.0_f32;
    let (ctop, cmid, cbot, border) = match state {
        KeycapState::Rest => (rgb(74, 75, 82), rgb(56, 57, 63), rgb(51, 52, 57), rgb(23, 23, 27)),
        KeycapState::PressedBlue => (rgb(47, 134, 196), rgb(30, 95, 146), rgb(26, 84, 128), rgb(13, 44, 68)),
        KeycapState::PressedAmber => (rgb(201, 122, 30), rgb(168, 100, 24), rgb(138, 78, 12), rgb(70, 40, 8)),
    };
    soft_shadow(p, rect, radius, 1.5, 2.5, 38.0);
    p.rect_filled(rect, radius, cmid);
    // gradient mesh inset ≥ r×0.42 so its square corners never poke past the border
    grad3(p, rect.shrink(2.0), ctop, cmid, cbot, 0.55);
    p.rect_stroke(rect, radius, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
    let pressed = matches!(state, KeycapState::PressedBlue | KeycapState::PressedAmber);
    let inset = radius + 1.0;
    let w = (rect.width() - 2.0 * inset).max(0.0);
    fade_line(p, egui::Rect::from_min_size(rect.min + egui::vec2(inset, 1.5), egui::vec2(w, 1.5)), (255, 255, 255), if pressed { 22 } else { 16 });
    fade_line(p, egui::Rect::from_min_size(egui::pos2(rect.left() + inset, rect.bottom() - 3.0), egui::vec2(w, 2.0)), (0, 0, 0), 44);
}

// ============================================================
// Pad — one step cell of the grid. Minimal FLAT look: a rounded rect in the
// state colour + a thin rounded border. Keyed by the cell's fill colour.
// ============================================================
pub fn pad(ui: &egui::Ui, rect: egui::Rect, fill: Color32) {
    let p = ui.painter();
    let r = 4.0;
    let (base, border) = if fill == BLUE() {
        (rgb(56, 150, 220), rgb(24, 86, 128))
    } else if fill == PL_LINK() {
        (rgb(228, 138, 40), rgb(115, 60, 10))
    } else if fill == SEQPL() {
        (rgb(158, 92, 222), rgb(74, 38, 128))
    } else if fill == CELL_EMPTY_BEAT() {
        (rgb(49, 50, 56), rgb(19, 19, 23))
    } else if fill == CELL_PL_LINK_OFF() {
        (rgb(50, 39, 24), rgb(74, 42, 4))
    } else if fill == CELL_PL_SNAP_OFF() {
        (rgb(50, 28, 24), rgb(74, 15, 8))
    } else if fill == CELL_SEQPL_OFF() {
        (rgb(44, 35, 54), rgb(44, 16, 80))
    } else if fill == FUSION_FILL() {
        (rgb(33, 55, 76), rgb(19, 19, 23))
    } else {
        (rgb(43, 44, 49), rgb(19, 19, 23))
    };
    p.rect_filled(rect, r, base);
    p.rect_stroke(rect, r, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
}

// ============================================================
// Slider track — the ONE renderer for every horizontal slider (header Len,
// track Vol/Hum/Push, sound-editor ENV). Recessed groove (dark pill + inner top
// shadow) + a pill value fill. `handle` = Some(radius) draws the round knob.
// ============================================================
pub fn slider_track(ui: &egui::Ui, track: egui::Rect, norm: f32, fill: Color32, cap: bool) {
    let p = ui.painter();
    let rr = track.height() * 0.5;
    // recessed groove: dark pill + a dark border + thin inner top shadow
    p.rect_filled(track, rr, rgb(16, 17, 21));
    p.rect_stroke(track, rr, egui::Stroke::new(1.0, rgb(9, 9, 12)), egui::StrokeKind::Inside);
    let mx = track.left() + rr;
    let gw = (track.width() - 2.0 * rr).max(0.0);
    p.rect_filled(
        egui::Rect::from_min_size(egui::pos2(mx, track.top() + 0.5), egui::vec2(gw, 1.5)),
        egui::epaint::CornerRadius::ZERO,
        Color32::from_black_alpha(130),
    );
    // value fill = a light bar inside the channel with rounded ends
    let x_val = track.left() + track.width() * norm;
    if norm > 0.0 {
        let ih = (track.height() - 2.0).max(2.0);
        let y = track.center().y;
        let x0 = track.left() + 1.0;
        if x_val - x0 >= ih {
            p.rect_filled(egui::Rect::from_min_max(egui::pos2(x0, y - ih * 0.5), egui::pos2(x_val, y + ih * 0.5)), ih * 0.5, fill);
        }
    }
    // striped fader cap at the value position — only for full sliders (`cap`),
    // never for the tiny Vol/Hum/Push mini-sliders (which stay a plain fill bar).
    if cap {
        fader_cap(p, x_val, track.center().y, (track.height() * 2.6).clamp(13.0, 20.0));
    }
}

/// Striped fader cap knob (mechanical grip): 3 hard horizontal bands, rounded
/// silhouette, soft drop shadow. `h` = cap height; width follows proportionally.
fn fader_cap(p: &egui::Painter, cx: f32, cy: f32, h: f32) {
    let w = (h * 0.62).max(8.0);
    let knob = egui::Rect::from_center_size(egui::pos2(cx, cy), egui::vec2(w, h));
    soft_shadow(p, knob, 3.0, 1.5, 2.0, 55.0);
    p.rect_filled(knob, 3.0, rgb(16, 16, 20)); // silhouette / border
    let k = knob.shrink(1.0);
    let (y0, hh) = (k.top(), k.height());
    let r_top = egui::epaint::CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 };
    let r_bot = egui::epaint::CornerRadius { nw: 0, ne: 0, sw: 2, se: 2 };
    p.rect_filled(egui::Rect::from_min_max(k.min, egui::pos2(k.right(), y0 + hh * 0.40)), r_top, rgb(96, 97, 105));
    p.rect_filled(egui::Rect::from_min_max(egui::pos2(k.left(), y0 + hh * 0.40), egui::pos2(k.right(), y0 + hh * 0.60)), egui::epaint::CornerRadius::ZERO, rgb(50, 51, 57));
    p.rect_filled(egui::Rect::from_min_max(egui::pos2(k.left(), y0 + hh * 0.60), k.max), r_bot, rgb(76, 77, 84));
}

// ============================================================
// Well recess — the shadow of a recessed panel (grid well, groove containers).
// Wall shadows on top + left + right that fade inward and hug the rounded
// corners; the floor (bottom) stays lit. Draw this AFTER the well's content.
// ============================================================
pub fn well_recess(ui: &egui::Ui, rect: egui::Rect, radius: f32) {
    let p = ui.painter_at(rect);
    inner_top_shadow(&p, rect, radius, 5.0, 150);
    inner_side_shadows(&p, rect, radius, 4.0, 120);
}

// ============================================================
// LCD screen background — the recessed green CRT glass behind the ADSR graph.
// Symmetric green gradient (lit from the centre) + recessed top shadow +
// green-black border + subtle scanlines. Draw this, then the curve on top.
// ============================================================
pub fn lcd_bg(ui: &egui::Ui, rect: egui::Rect, radius: f32) {
    let p = ui.painter();
    // dark rounded base → clean AA corners under the (square) gradient mesh
    p.rect_filled(rect, radius, rgb(7, 13, 9));
    grad3(p, rect.shrink(2.0), rgb(9, 16, 11), rgb(16, 27, 20), rgb(9, 16, 11), 0.5);
    // recessed top shadow (inset from the corners so it doesn't poke)
    let ins = radius.max(1.0);
    p.rect_filled(
        egui::Rect::from_min_size(egui::pos2(rect.left() + ins, rect.top() + 1.0), egui::vec2((rect.width() - 2.0 * ins).max(0.0), 3.0)),
        egui::epaint::CornerRadius::ZERO,
        Color32::from_black_alpha(80),
    );
    // green-black border (the frame is part of the screen)
    p.rect_stroke(rect, radius, egui::Stroke::new(1.0, rgb(5, 9, 6)), egui::StrokeKind::Inside);
    // subtle CRT scanlines
    let inner = rect.shrink(2.0);
    let mut y = inner.top() + 1.0;
    while y < inner.bottom() {
        p.line_segment([egui::pos2(inner.left(), y), egui::pos2(inner.right(), y)], egui::Stroke::new(1.0, Color32::from_black_alpha(28)));
        y += 3.0;
    }
}
