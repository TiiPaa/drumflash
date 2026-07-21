use crate::ui::slider::{denormalize_value, normalize_value};
use nih_plug_egui::egui::emath::GuiRounding;
use nih_plug_egui::egui::{self, Color32, Response, Sense, Ui, Vec2, Widget};
use std::ops::RangeInclusive;

/// When shift+dragging a parameter, one pixel dragged corresponds to this much change in the
/// normalized parameter.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.0015;

/// A slider widget similar to egui::Slider that supports granular drag for fine-tuning.
/// This is designed for local mutable values (like plock parameters) that aren't NIH-plug Params.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct LocalParamSlider<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    logarithmic: bool,
    suffix: Option<&'a str>,
    draw_value: bool,
    slider_width: Option<f32>,
    reset_value: Option<f32>,
}

impl<'a> LocalParamSlider<'a> {
    /// Create a new slider for a local mutable value.
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            logarithmic: false,
            suffix: None,
            draw_value: true,
            slider_width: None,
            reset_value: None,
        }
    }

    /// Set logarithmic scaling for the slider.
    pub fn logarithmic(mut self, logarithmic: bool) -> Self {
        self.logarithmic = logarithmic;
        self
    }

    #[allow(dead_code)]
    /// Set a suffix to display after the value.
    pub fn suffix(mut self, suffix: &'a str) -> Self {
        self.suffix = Some(suffix);
        self
    }

    /// Don't draw the text slider's current value after the slider.
    pub fn without_value(mut self) -> Self {
        self.draw_value = false;
        self
    }

    /// Set a custom width for the slider.
    pub fn with_width(mut self, width: f32) -> Self {
        self.slider_width = Some(width);
        self
    }

    /// Set the value used on double-click reset.
    pub fn reset_value(mut self, value: f32) -> Self {
        self.reset_value = Some(value);
        self
    }

    fn normalized_value(&self) -> f32 {
        normalize_value(
            *self.value,
            *self.range.start(),
            *self.range.end(),
            self.logarithmic,
        )
    }

    fn set_normalized_value(&mut self, normalized: f32) {
        *self.value = denormalize_value(
            normalized,
            *self.range.start(),
            *self.range.end(),
            self.logarithmic,
        );
    }

    fn string_value(&self) -> String {
        if let Some(suffix) = self.suffix {
            format!("{:.2}{}", self.value, suffix)
        } else {
            format!("{:.2}", self.value)
        }
    }

    fn granular_drag(&mut self, _ui: &Ui, drag_delta: Vec2) {
        // For granular drag, we work directly with the normalized value
        let current_normalized = self.normalized_value();
        let drag_amount = drag_delta.x * GRANULAR_DRAG_MULTIPLIER;
        self.set_normalized_value(current_normalized + drag_amount);
    }
}

impl<'a> Widget for LocalParamSlider<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let slider_width = self
            .slider_width
            .unwrap_or_else(|| ui.spacing().slider_width);

        // Use the same height calculation as ParamSlider for consistency
        let height = ui
            .text_style_height(&egui::TextStyle::Body)
            .max(ui.spacing().interact_size.y * 0.8);
        let slider_height = (height * 0.8).round_to_pixels(ui.painter().pixels_per_point());

        let mut response = ui.allocate_response(
            egui::vec2(slider_width, slider_height),
            Sense::click_and_drag(),
        );

        // Handle user input
        if response.drag_started() {
            // Reset drag state when starting a new drag
        }

        if let Some(click_pos) = response.interact_pointer_pos() {
            if ui.input(|i| i.modifiers.shift) {
                // Shift+drag for granular/fine-tuning
                self.granular_drag(ui, response.drag_delta());
                response.mark_changed();
            } else {
                // Normal drag - map click position to normalized value
                let proportion =
                    egui::emath::remap_clamp(click_pos.x, response.rect.x_range(), 0.0..=1.0)
                        as f64;
                self.set_normalized_value(proportion as f32);
                response.mark_changed();
            }
        }

        if response.double_clicked() {
            // Double-click to reset to default (middle of range unless overridden)
            *self.value = self
                .reset_value
                .unwrap_or_else(|| (self.range.start() + self.range.end()) / 2.0);
            response.mark_changed();
        }

        // Draw the slider
        if ui.is_rect_visible(response.rect) {
            // Background
            ui.painter()
                .rect_filled(response.rect, 0.0, ui.visuals().widgets.inactive.bg_fill);

            // Filled portion
            let filled_proportion = self.normalized_value();
            if filled_proportion > 0.0 {
                let mut filled_rect = response.rect;
                filled_rect.set_width(response.rect.width() * filled_proportion);
                let filled_bg = if response.dragged() {
                    // Slightly brighter when dragging
                    let mut hsv =
                        egui::epaint::Hsva::from(egui::Rgba::from(ui.visuals().selection.bg_fill));
                    hsv.v += 0.1;
                    hsv.a = 1.0;
                    egui::Color32::from(hsv)
                } else {
                    ui.visuals().selection.bg_fill
                };
                ui.painter().rect_filled(filled_rect, 0.0, filled_bg);
            }

            // Border
            ui.painter().rect_stroke(
                response.rect,
                0.0,
                egui::Stroke::new(1.0, ui.visuals().widgets.active.bg_fill),
                egui::StrokeKind::Middle,
            );
        }

        // Draw the value text if enabled
        if self.draw_value {
            let text = self.string_value();
            let text_galley = ui.fonts(|f| {
                f.layout_no_wrap(
                    text,
                    egui::TextStyle::Button.resolve(ui.style()),
                    Color32::WHITE,
                )
            });
            let text_size = text_galley.size();
            let padding = ui.spacing().button_padding;

            let text_response = ui.allocate_response(text_size + (padding * 2.0), Sense::click());

            if ui.is_rect_visible(text_response.rect) {
                let text_pos = ui
                    .layout()
                    .align_size_within_rect(text_size, text_response.rect.shrink2(padding))
                    .min;

                ui.painter().add(egui::epaint::TextShape::new(
                    text_pos,
                    text_galley,
                    ui.visuals().widgets.inactive.fg_stroke.color,
                ));
            }
        }

        response
    }
}
