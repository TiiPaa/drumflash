//! Unified slider logic and track rendering.
//!
//! Shared by the Sound Editor rows, the grid-lane mini sliders and the plock
//! menu's `LocalParamSlider`. The header slider stays param-bound and is not
//! part of this module.

use crate::ui::theme::*;
use nih_plug_egui::egui::{self, Color32, Response, Sense, Ui, Vec2};

/// Normalize a plain value into 0..=1 (linear or logarithmic).
/// Falls back to linear mapping when the logarithmic domain is invalid.
pub fn normalize_value(value: f32, min: f32, max: f32, logarithmic: bool) -> f32 {
    if logarithmic && min > 0.0 && max > min {
        let min_log = min.ln();
        let max_log = max.ln();
        ((value.max(min).ln() - min_log) / (max_log - min_log)).clamp(0.0, 1.0)
    } else {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    }
}

/// Denormalize a 0..=1 position back to a plain value (linear or logarithmic).
/// Falls back to linear mapping when the logarithmic domain is invalid.
pub fn denormalize_value(norm: f32, min: f32, max: f32, logarithmic: bool) -> f32 {
    let norm = norm.clamp(0.0, 1.0);
    if logarithmic && min > 0.0 && max > min {
        (min.ln() + norm * (max.ln() - min.ln())).exp()
    } else {
        min + norm * (max - min)
    }
}

/// Normalized units travelled per pixel of fine drag. Mirrors the plock menu's
/// `GRANULAR_DRAG_MULTIPLIER` so both fine-tune gestures feel the same — about
/// 4x finer than the absolute mapping on a typical editor row.
pub const FINE_DRAG_NORM_PER_PX: f32 = 0.0015;

/// Fine-tune drag: move `value` RELATIVE to itself instead of jumping to the
/// pointer ([181]). Kept separate from `draw_track` so the maths is unit-testable
/// without an egui context.
pub fn apply_fine_drag(
    value: &mut f32,
    delta_px: f32,
    min: f32,
    max: f32,
    logarithmic: bool,
    step: f32,
) -> bool {
    if delta_px == 0.0 {
        return false;
    }
    let norm = normalize_value(*value, min, max, logarithmic);
    let target = (norm + delta_px * FINE_DRAG_NORM_PER_PX).clamp(0.0, 1.0);
    let mut new_value = denormalize_value(target, min, max, logarithmic).clamp(min, max);
    if step > 0.0 {
        new_value = ((new_value / step).round() * step).clamp(min, max);
    }
    if new_value == *value {
        return false;
    }
    *value = new_value;
    true
}

/// Visual options for `draw_track`. Both call sites (editor rows and grid
/// lanes) keep their exact previous look by passing their own values.
#[derive(Clone, Copy)]
pub struct TrackStyle {
    /// Total widget height (allocation).
    pub height: f32,
    /// Thickness of the drawn track bar.
    pub track_h: f32,
    /// Fill color of the value portion.
    pub fill: Color32,
    /// Draw a striped fader cap at the value (full sliders) vs a plain fill bar
    /// (tiny Vol/Hum/Push mini-sliders).
    pub cap: bool,
    /// Quantisation step (0 = continuous). The dragged value snaps to
    /// multiples of this step, e.g. 1.0 for integer semitones.
    pub step: f32,
}

impl TrackStyle {
    pub fn editor() -> Self {
        Self {
            height: 22.0,
            track_h: 6.0,
            fill: BLUE(),
            cap: true,
            step: 0.0,
        }
    }

    pub fn mini() -> Self {
        Self {
            height: 17.0,
            track_h: 6.0,
            fill: BLUE(),
            cap: false,
            step: 0.0,
        }
    }

    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }
}

/// Core interactive slider track: click/drag to set, double-click to reset,
/// Shift/Alt+drag for fine-tune ([181]).
/// Returns the response with `changed` marked when the value was modified.
pub fn draw_track(
    ui: &mut Ui,
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    logarithmic: bool,
    width: f32,
    style: TrackStyle,
) -> Response {
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width, style.height), Sense::click_and_drag());
    let fine = crate::ui::controls::fine_tune_modifier_pressed(ui);
    if let Some(pos) = response.interact_pointer_pos() {
        if fine {
            // Fine-tune: relative drag from the CURRENT value, and a bare
            // Shift/Alt click does nothing (jumping would defeat the purpose).
            if response.dragged()
                && apply_fine_drag(
                    value,
                    response.drag_delta().x,
                    min,
                    max,
                    logarithmic,
                    style.step,
                )
            {
                response.mark_changed();
            }
        } else if response.clicked() || response.dragged() {
            let norm = egui::emath::remap_clamp(pos.x, rect.x_range(), 0.0..=1.0);
            *value = denormalize_value(norm, min, max, logarithmic).clamp(min, max);
            if style.step > 0.0 {
                *value = (*value / style.step).round() * style.step;
                *value = value.clamp(min, max);
            }
            response.mark_changed();
        }
    }
    if response.double_clicked() {
        *value = default.clamp(min, max);
        response.mark_changed();
    }

    let track = egui::Rect::from_center_size(rect.center(), Vec2::new(rect.width(), style.track_h));
    let norm = normalize_value(*value, min, max, logarithmic);
    // All slider visuals live in one place: `skeuo::slider_track`.
    crate::ui::skeuo::slider_track(ui, track, norm, style.fill, style.cap);

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_roundtrip() {
        for v in [0.0f32, 0.25, 0.5, 0.75, 1.0, 2.0] {
            let norm = normalize_value(v, 0.0, 2.0, false);
            let back = denormalize_value(norm, 0.0, 2.0, false);
            assert!((back - v).abs() < 1e-6, "v={} norm={} back={}", v, norm, back);
        }
    }

    #[test]
    fn logarithmic_roundtrip() {
        for v in [20.0f32, 100.0, 1000.0, 8000.0, 20000.0] {
            let norm = normalize_value(v, 20.0, 20000.0, true);
            let back = denormalize_value(norm, 20.0, 20000.0, true);
            assert!((back - v).abs() / v < 1e-4, "v={} norm={} back={}", v, norm, back);
        }
    }

    #[test]
    fn fine_drag_moves_relative_and_far_slower_than_absolute() {
        // 100 px of fine drag on a 0..1 range moves 0.15 — a 200 px absolute
        // track would have moved 0.5 over the same distance.
        let mut v = 0.5f32;
        assert!(apply_fine_drag(&mut v, 100.0, 0.0, 1.0, false, 0.0));
        assert!((v - 0.65).abs() < 1e-5, "v={v}");
        // ...and backwards from wherever it now is.
        assert!(apply_fine_drag(&mut v, -100.0, 0.0, 1.0, false, 0.0));
        assert!((v - 0.5).abs() < 1e-5, "v={v}");
    }

    #[test]
    fn fine_drag_clamps_at_the_bounds_and_reports_no_change() {
        let mut v = 1.0f32;
        assert!(!apply_fine_drag(&mut v, 500.0, 0.0, 1.0, false, 0.0));
        assert_eq!(v, 1.0);
        let mut v = 0.0f32;
        assert!(!apply_fine_drag(&mut v, -500.0, 0.0, 1.0, false, 0.0));
        assert_eq!(v, 0.0);
        // A zero-pixel drag is not a change either.
        let mut v = 0.5f32;
        assert!(!apply_fine_drag(&mut v, 0.0, 0.0, 1.0, false, 0.0));
    }

    #[test]
    fn fine_drag_follows_the_logarithmic_mapping() {
        // On a log range the same pixel delta moves proportionally, not linearly.
        let mut v = 100.0f32;
        assert!(apply_fine_drag(&mut v, 50.0, 20.0, 20000.0, true, 0.0));
        let ratio = v / 100.0;
        let expected = (20000.0f32 / 20.0).powf(50.0 * FINE_DRAG_NORM_PER_PX);
        assert!((ratio - expected).abs() < 1e-3, "ratio={ratio} expected={expected}");
    }

    #[test]
    fn fine_drag_honours_the_quantisation_step() {
        // Stepped sliders (integer semitones) stay on their grid: a small drag
        // that cannot reach the next step reports no change.
        let mut v = 5.0f32;
        assert!(!apply_fine_drag(&mut v, 1.0, 0.0, 24.0, false, 1.0));
        assert_eq!(v, 5.0);
        assert!(apply_fine_drag(&mut v, 30.0, 0.0, 24.0, false, 1.0));
        assert_eq!(v, 6.0);
    }

    #[test]
    fn clamping_out_of_range() {
        assert_eq!(normalize_value(-5.0, 0.0, 10.0, false), 0.0);
        assert_eq!(normalize_value(50.0, 0.0, 10.0, false), 1.0);
        assert_eq!(denormalize_value(-1.0, 0.0, 10.0, false), 0.0);
        assert_eq!(denormalize_value(2.0, 0.0, 10.0, false), 10.0);
    }

    #[test]
    fn logarithmic_falls_back_to_linear_when_domain_invalid() {
        // min <= 0 makes ln() invalid; mapping must stay linear.
        let norm = normalize_value(5.0, 0.0, 10.0, true);
        assert!((norm - 0.5).abs() < 1e-6);
        let back = denormalize_value(0.5, 0.0, 10.0, true);
        assert!((back - 5.0).abs() < 1e-6);
    }

    #[test]
    fn logarithmic_anchors_endpoints() {
        assert_eq!(normalize_value(20.0, 20.0, 20000.0, true), 0.0);
        assert_eq!(normalize_value(20000.0, 20.0, 20000.0, true), 1.0);
    }
}
