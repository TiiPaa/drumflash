//! Bottom panel: Generator | Song tabs, generator controls, preset chips.

use crate::generator::{self, GeneratorType, Style};
use crate::pattern_bank::SONG_BLOCKS;
use crate::sequencer::{Pattern, SharedPattern};
use crate::ui::controls::{
    chip_button, compact_chip, enum_combo_compact, generator_song_segmented, genrow_label,
};
use crate::ui::editor_state::EditorUIState;
use crate::ui::header::header_param_slider;
use crate::ui::pattern_bank::load_pattern_for_ui_with_length;
use crate::ui::song::draw_song_editor;
use crate::ui::theme::*;
use crate::DrumFlashParams;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText, Vec2};
use std::sync::{
    atomic::{AtomicBool, AtomicU32},
    Arc,
};

pub fn draw_bottom_panel(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
    _song_mode: &Arc<AtomicBool>,
    song_position: &Arc<AtomicU32>,
) {
    let panel_w = ui.available_width();
    // Compact height sized for the 800px window (designer target): the
    // Generator view needs ~70px of body and the Song editor ~110px.
    let panel_h = 168.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(panel_w, panel_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, RADIUS_PANEL, PANEL());
    painter.rect_stroke(
        rect,
        RADIUS_PANEL,
        egui::Stroke::new(1.0, LINE()),
        egui::StrokeKind::Inside,
    );
    const BOTTOM_HEADER_H: f32 = 42.0;
    const BOTTOM_SEPARATOR_Y: f32 = 42.0;
    const BOTTOM_BODY_TOP: f32 = 44.0;
    const BOTTOM_PAD_X: f32 = 12.0;

    painter.hline(
        rect.x_range(),
        rect.top() + BOTTOM_SEPARATOR_Y,
        egui::Stroke::new(1.0, LINE()),
    );

    let header_rect = egui::Rect::from_min_size(rect.min, Vec2::new(panel_w, BOTTOM_HEADER_H));
    let switch_top = rect.top() + ((BOTTOM_HEADER_H - CTL_HEIGHT) * 0.5).floor();
    let switch_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left() + BOTTOM_PAD_X, switch_top),
        Vec2::new(150.0, CTL_HEIGHT),
    );
    let selected = state.bottom_panel_tab.min(1);
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(switch_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
        |ui| {
            // Generator | Song segmented tabs: view only, no longer toggle song mode.
            ui.set_clip_rect(header_rect);
            ui.spacing_mut().item_spacing.x = 0.0;
            let new_selected = generator_song_segmented(ui, selected);
            if new_selected != selected {
                state.bottom_panel_tab = new_selected;
            }
        },
    );

    let meta = if state.bottom_panel_tab == 1 {
        if let Ok(bank) = params.pattern_bank.bank.lock() {
            let blocks = (bank.song.length as usize).min(SONG_BLOCKS);
            let total_reps = bank.song.steps[..blocks]
                .iter()
                .filter(|&&s| s >= 0)
                .count();
            format!("{} blocks · {} patterns", blocks, total_reps)
        } else {
            "Song chain".to_string()
        }
    } else {
        format!(
            "{} · {} -> {}",
            GeneratorType::variants()[params.generator_type.value().to_index()],
            Style::variants()[params.style_primary.value().to_index()],
            Style::variants()[params.style_secondary.value().to_index()]
        )
    };
    painter.text(
        egui::pos2(rect.left() + BOTTOM_PAD_X + 142.0, switch_rect.center().y),
        egui::Align2::LEFT_CENTER,
        meta,
        f_mono_med(10.5),
        INK3(),
    );

    let body_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + BOTTOM_PAD_X, rect.top() + BOTTOM_BODY_TOP),
        egui::pos2(rect.right() - BOTTOM_PAD_X, rect.bottom() - 8.0),
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(body_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.set_clip_rect(body_rect);
            ui.set_width(body_rect.width());
            ui.set_height(body_rect.height());
            if state.bottom_panel_tab == 1 {
                draw_song_editor(ui, setter, params, state, song_position);
            } else {
                draw_generator_panel_content(ui, setter, params, pattern, state);
            }
        },
    );
}

fn draw_generator_panel_content(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
) {
    ui.vertical(|ui| {
        ui.add_space(6.0);
        ui.spacing_mut().item_spacing.y = 12.0;
        draw_generator_bar(ui, setter, params, pattern, state);
        draw_preset_bar(ui, pattern, params, setter, state);
    });
}

fn draw_preset_bar(
    ui: &mut egui::Ui,
    pattern: &SharedPattern,
    params: &DrumFlashParams,
    _setter: &ParamSetter,
    state: &mut EditorUIState,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 7.0;
        ui.set_height(CTL_HEIGHT);
        genrow_label(ui, "Presets", 62.0);
        let pattern_length = params.pattern_length.value() as usize;
        if compact_chip(ui, "Rock", false).clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            crate::ui::grid::clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::rock_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        if compact_chip(ui, "Funk", false).clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            crate::ui::grid::clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::funk_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        if compact_chip(ui, "Disco", false).clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            crate::ui::grid::clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::disco_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
        ui.add_space(8.0);
        if chip_button(ui, "⟳ Random", true, PL_LINK(), egui::Sense::click()).clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            crate::ui::grid::clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &Pattern::random_pattern(), pattern_length);
            state.last_loaded_slot = None;
        }
    });
}

fn draw_generator_bar(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &DrumFlashParams,
    pattern: &SharedPattern,
    state: &mut EditorUIState,
) {
    const GEN_TYPE_W: f32 = 116.0;
    const STYLE_W: f32 = 70.0;
    const GEN_BTN_W: f32 = 104.0;

    // Single aligned row: algorithm · style A → B · Mix/Dens/Var · GENERATE (right).
    ui.horizontal(|ui| {
        ui.set_height(CTL_HEIGHT);
        ui.spacing_mut().item_spacing.x = 0.0;

        // Generator algorithm (Probabilistic / Euclidean / Markov / …)
        enum_combo_compact(ui, setter, &params.generator_type, "gen_type", GEN_TYPE_W);

        // Style morph A → B
        ui.add_space(16.0);
        genrow_label(ui, "A", 12.0);
        ui.add_space(5.0);
        enum_combo_compact(ui, setter, &params.style_primary, "style_a", STYLE_W);
        ui.add_space(12.0);
        genrow_label(ui, "B", 12.0);
        ui.add_space(5.0);
        enum_combo_compact(ui, setter, &params.style_secondary, "style_b", STYLE_W);

        // Amounts (design-system pill sliders)
        ui.add_space(18.0);
        const SLIDER_TOTAL_W: f32 = 110.0;
        header_param_slider(
            ui,
            setter,
            &params.style_mix,
            SLIDER_TOTAL_W,
            "Mix A/B",
            false,
        );
        ui.add_space(10.0);
        header_param_slider(
            ui,
            setter,
            &params.gen_density,
            SLIDER_TOTAL_W,
            "Density",
            false,
        );
        ui.add_space(10.0);
        header_param_slider(
            ui,
            setter,
            &params.gen_variation,
            SLIDER_TOTAL_W,
            "Variation",
            false,
        );

        // GENERATE, pushed to the right edge
        let space = (ui.available_width() - GEN_BTN_W).max(10.0);
        ui.add_space(space);
        let gen_btn_response = ui.add_sized(
            Vec2::new(GEN_BTN_W, CTL_HEIGHT),
            egui::Button::new(
                RichText::new("GENERATE")
                    .font(f_sans_sb(11.0))
                    .color(Color32::WHITE),
            )
            .fill(BLUE())
            .stroke(egui::Stroke::new(1.0, BLUE()))
            .corner_radius(RADIUS_CTL),
        );

        if gen_btn_response.clicked() {
            params.plock_state.state.clear_all();
            params.seq_plock_state.state.clear_all();
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0);
            let gen_params = generator::GeneratorParams {
                generator_type: params.generator_type.value(),
                style_primary: params.style_primary.value(),
                style_secondary: params.style_secondary.value(),
                style_mix: params.style_mix.value(),
                density: params.gen_density.value(),
                variation: params.gen_variation.value(),
                seed,
            };
            let generated = generator::generate(&gen_params, params.track_layout.state.as_ref());
            let pattern_length = params.pattern_length.value() as usize;
            crate::ui::grid::clear_all_fusions(pattern);
            load_pattern_for_ui_with_length(pattern, &generated, pattern_length);
            state.last_loaded_slot = None;
        }
    });
}
