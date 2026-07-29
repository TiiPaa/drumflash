//! Sequencer step cells rendered from the designer's **baked bitmap atlas**
//! (`assets/pads/atlas-pads.png` + `atlas-pads.json`).
//!
//! 69 sprites: 9 simple steps + 60 fused-cell lengths (`fuse-<status>-<N>`,
//! N = 2..16). Each atlas rect includes an 8-pt transparent bleed on all sides
//! (so shadows/glow are captured); the useful element (`ew × eh`) is centred.
//! To draw a cell we expand its logical rect by the (scaled) bleed and blit the
//! whole sprite there. Overlays (playhead ring, fusion pulse count) are NOT baked
//! and stay drawn on top by the grid; out-of-range dims the sprite (tint 28 %).
//!
//! The plugin's cells are a touch shorter than the baked 26 pt, so the sprite is
//! blitted scaled to the real cell rect (baked ×4 → downscales crisp).

use crate::ui::theme::*;
use nih_plug_egui::egui::{self, Color32, ImageSource};
use std::collections::HashMap;
use std::sync::OnceLock;

const ATLAS_JSON: &str = include_str!("../../assets/pads/atlas-pads.json");
const ATLAS_W: f32 = 1600.0;
const ATLAS_H: f32 = 840.0;

fn atlas_source() -> ImageSource<'static> {
    // Embedded once; egui caches the decoded texture by URI, so per-cell calls
    // just reference the cached atlas.
    egui::include_image!("../../assets/pads/atlas-pads.png")
}

/// `name -> [x, y, w, h, ew, eh]` parsed once from the manifest.
fn sprites() -> &'static HashMap<String, [f32; 6]> {
    static MAP: OnceLock<HashMap<String, [f32; 6]>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(ATLAS_JSON) {
            if let Some(obj) = v.get("sprites").and_then(|s| s.as_object()) {
                for (name, arr) in obj {
                    if let Some(a) = arr.as_array() {
                        if a.len() == 6 {
                            let mut r = [0.0f32; 6];
                            for (i, n) in a.iter().enumerate() {
                                r[i] = n.as_f64().unwrap_or(0.0) as f32;
                            }
                            m.insert(name.clone(), r);
                        }
                    }
                }
            }
        }
        m
    })
}

/// Lit-status token for a fill colour, or `None` for off states.
fn hit_status(fill: Color32) -> Option<&'static str> {
    if fill == BLUE() {
        Some("hit")
    } else if fill == PL_LINK() {
        Some("hit-link")
    } else if fill == SEQPL() {
        Some("hit-seq")
    } else {
        None
    }
}

/// Atlas sprite name for a cell's state + fusion span.
fn sprite_name(fill: Color32, fusion_span: Option<usize>) -> String {
    if let Some(n) = fusion_span {
        if n >= 2 {
            // Fused cells are always active → a hit status; clamp N to 2..=16.
            let status = hit_status(fill).unwrap_or("hit");
            return format!("fuse-{}-{}", status, n.clamp(2, 16));
        }
    }
    if fill == BLUE() {
        "pad-hit".into()
    } else if fill == PL_LINK() {
        "pad-hit-link".into()
    } else if fill == SEQPL() {
        "pad-hit-seq".into()
    } else if fill == CELL_EMPTY_BEAT() {
        "pad-off-beat".into()
    } else if fill == CELL_PL_LINK_OFF() {
        "pad-off-link".into()
    } else if fill == CELL_PL_SNAP_OFF() {
        "pad-off-snap".into()
    } else if fill == CELL_SEQPL_OFF() {
        "pad-off-seq".into()
    } else {
        "pad-off".into()
    }
}

/// Blit the atlas sprite for this cell into `cell_rect` (bleed-corrected, scaled
/// to the real cell size). `in_range == false` dims the sprite (out-of-length).
pub fn draw_pad(ui: &egui::Ui, cell_rect: egui::Rect, fill: Color32, fusion_span: Option<usize>, in_range: bool) {
    let name = sprite_name(fill, fusion_span);
    let Some(&[x, y, w, h, ew, eh]) = sprites().get(&name) else {
        // Fallback: never leave a cell invisible if a sprite is missing.
        ui.painter().rect_filled(cell_rect, 4.0, fill);
        return;
    };
    // The atlas was baked with an OPAQUE dark background (the 8-pt "bleed" is not
    // transparent), so painting the full frame lets each cell's bleed overwrite
    // its neighbour's edge (few-pixel truncation on the right + between lanes).
    // We therefore blit ONLY the useful element (ew×eh, bleed cropped) straight
    // into the cell rect — no overlap. Trade-off: the shadow/glow spill baked into
    // the bleed is dropped (re-bake with a real alpha background to restore it).
    let bx = (w - ew) * 0.5;
    let by = (h - eh) * 0.5;
    let uv = egui::Rect::from_min_max(
        egui::pos2((x + bx) / ATLAS_W, (y + by) / ATLAS_H),
        egui::pos2((x + bx + ew) / ATLAS_W, (y + by + eh) / ATLAS_H),
    );
    let tint = if in_range {
        Color32::WHITE
    } else {
        Color32::WHITE.gamma_multiply(0.28)
    };
    // Round the blit to soften the pad corners. The atlas already bakes ~4px
    // corners but the area outside them is the opaque dark bg (≈ well colour), so
    // a small radius is invisible — we clip a touch INTO the pad body (6px) so the
    // rounding actually reads.
    egui::Image::new(atlas_source())
        .uv(uv)
        .tint(tint)
        .corner_radius(egui::CornerRadius::same(6))
        .paint_at(ui, cell_rect);
}
