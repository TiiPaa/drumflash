//! Skeuomorphic rubber-pad textures for the step grid.
//!
//! The "hardware" look of the pads comes from the designer's exported PNGs
//! (`assets/pads/pad-*.png`). We render them with the designer's own recipe
//! (`rust/skeuo_widgets.rs::pad`): an `egui::Image` from an `include_image!`
//! source, rounded to the pad radius via `corner_radius`. The PNG loader is
//! installed once at editor startup (`egui_extras::install_image_loaders`).

use crate::ui::theme::*;
use nih_plug_egui::egui::{self, Color32, ImageSource};

/// PNG pad source for a cell's state colour, or `None` for states that keep
/// vector rendering (fusion blocks, editing pulse, selection, disabled).
pub fn pad_source_for(fill: Color32) -> Option<ImageSource<'static>> {
    let src = if fill == BLUE() {
        egui::include_image!("../../assets/pads/pad-hit.png")
    } else if fill == PL_LINK() {
        egui::include_image!("../../assets/pads/pad-hit-link.png")
    } else if fill == SEQPL() {
        egui::include_image!("../../assets/pads/pad-seq-hit.png")
    } else if fill == CELL_EMPTY_BEAT() {
        egui::include_image!("../../assets/pads/pad-off-beat.png")
    } else if fill == CELL_EMPTY_OFF() || fill == CELL_CURRENT() {
        // Playhead-on-empty keeps the plain off pad; the ring is drawn on top.
        egui::include_image!("../../assets/pads/pad-off.png")
    } else if fill == CELL_PL_LINK_OFF() {
        egui::include_image!("../../assets/pads/pad-off-link.png")
    } else if fill == CELL_PL_SNAP_OFF() {
        egui::include_image!("../../assets/pads/pad-off-snap.png")
    } else if fill == CELL_SEQPL_OFF() {
        egui::include_image!("../../assets/pads/pad-seq-off.png")
    } else {
        return None;
    };
    Some(src)
}
