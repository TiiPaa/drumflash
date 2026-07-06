//! Interactive envelope visualizer for the Sound Panel.
//!
//! Two separate views:
//! - `draw_amp_envelope`   : ADSR-style amplitude curve (amber/blue/purple)
//! - `draw_filter_envelope`: Filter envelope curve (orange)
//!
//! The engine is bi-stage parallel (DecayReleaseEnvelope), but the amplitude
//! graph intentionally follows the designer mockup's simplified ADSR readout.

use crate::ui::theme::*;
use nih_plug_egui::egui::{Color32, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};

// -- Amplitude envelope (AHDSR style) ----------------------------------------

pub fn draw_amp_envelope(
    ui: &mut nih_plug_egui::egui::Ui,
    attack_time: f32,
    decay: f32,
    decay_curve: f32,
    hold: f32,
    release: f32,
    release_curve: f32,
    has_release: bool,
) -> nih_plug_egui::egui::Response {
    let w = ui.available_width().max(120.0);
    let desired_size = Vec2::new(w, 104.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 7.0, Color32::from_rgb(12, 12, 17));
    painter.rect_stroke(rect, 7.0, Stroke::new(1.0, LINE), StrokeKind::Inside);

    let pad_x = 12.0f32;
    let pad_y = 12.0f32;
    let graph = Rect::from_min_size(
        rect.min + Vec2::new(pad_x, pad_y),
        rect.size() - Vec2::new(pad_x * 2.0, pad_y * 2.0),
    );

    for i in 0..=4 {
        let x = graph.min.x + graph.width() * i as f32 / 4.0;
        painter.line_segment(
            [Pos2::new(x, graph.min.y), Pos2::new(x, graph.max.y)],
            Stroke::new(1.0, white_a(13)),
        );
    }

    let attack = attack_time.max(0.02);
    let decay_time = (hold + decay).max(0.05);
    let release_time = release.max(0.05);
    let total_time = if has_release {
        attack + decay_time + release_time
    } else {
        attack + decay_time
    };

    let base_y = graph.max.y;
    let top_y = graph.min.y;
    let sustain_y = graph.min.y + graph.height() * 0.62;
    let x_start = graph.min.x;
    let x_attack = graph.min.x + graph.width() * attack / total_time;
    let x_decay = if has_release {
        graph.min.x + graph.width() * (attack + decay_time) / total_time
    } else {
        graph.max.x
    };
    let x_end = graph.max.x;

    painter.line_segment(
        [Pos2::new(x_start, base_y), Pos2::new(x_attack, top_y)],
        Stroke::new(2.0, AMBER),
    );
    draw_curve(
        &painter,
        Pos2::new(x_attack, top_y),
        Pos2::new(x_decay, if has_release { sustain_y } else { base_y }),
        decay_curve,
        BLUE,
    );
    if has_release {
        draw_curve(
            &painter,
            Pos2::new(x_decay, sustain_y),
            Pos2::new(x_end, base_y),
            release_curve,
            SEQPL,
        );
    }

    let label_color = white_a(150);
    draw_env_label(
        &painter,
        graph,
        "A",
        Pos2::new(x_attack - 3.0, base_y - 3.0),
        label_color,
    );
    draw_env_label(
        &painter,
        graph,
        "D",
        Pos2::new(x_attack + (x_decay - x_attack) * 0.55, sustain_y - 8.0),
        label_color,
    );
    if has_release {
        draw_env_label(
            &painter,
            graph,
            "S",
            Pos2::new(x_decay + 4.0, sustain_y - 4.0),
            label_color,
        );
        draw_env_label(
            &painter,
            graph,
            "R",
            Pos2::new(x_end - 12.0, base_y - 3.0),
            label_color,
        );
    }

    response
}

fn draw_env_label(
    painter: &nih_plug_egui::egui::Painter,
    graph: Rect,
    label: &str,
    pos: Pos2,
    color: Color32,
) {
    let pos = Pos2::new(
        pos.x.clamp(graph.min.x + 2.0, graph.max.x - 10.0),
        pos.y.clamp(graph.min.y + 10.0, graph.max.y - 2.0),
    );
    painter.text(
        pos,
        nih_plug_egui::egui::Align2::LEFT_BOTTOM,
        label,
        f_mono_med(10.0),
        color,
    );
}

fn draw_curve(
    painter: &nih_plug_egui::egui::Painter,
    start: Pos2,
    end: Pos2,
    curve: f32,
    color: Color32,
) {
    const POINTS: usize = 40;
    let exponent = 1.0 + (curve.max(0.0) / 2.0);
    let mut points = Vec::with_capacity(POINTS + 1);
    for i in 0..=POINTS {
        let t = i as f32 / POINTS as f32;
        let eased = t.powf(exponent);
        let x = start.x + (end.x - start.x) * t;
        let y = start.y + (end.y - start.y) * eased;
        points.push(Pos2::new(x, y));
    }
    painter.add(Shape::line(points, Stroke::new(2.0, color)));
}

// -- Filter envelope ---------------------------------------------------------

pub fn draw_filter_envelope(
    ui: &mut nih_plug_egui::egui::Ui,
    decay_curve: f32,
    filter_env_decay: f32,
) -> nih_plug_egui::egui::Response {
    let w = ui.available_width().max(120.0);
    let desired_size = Vec2::new(w, 104.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 7.0, Color32::from_rgb(12, 12, 17));
    painter.rect_stroke(rect, 7.0, Stroke::new(1.0, LINE), StrokeKind::Inside);

    let pad_x = 14.0f32;
    let pad_y = 10.0f32;
    let graph = Rect::from_min_size(
        rect.min + Vec2::new(pad_x, pad_y),
        rect.size() - Vec2::new(pad_x * 2.0, pad_y * 2.0),
    );

    let total_time = filter_env_decay.max(0.1);
    const POINTS: usize = 200;
    let mut points: Vec<Pos2> = Vec::with_capacity(POINTS + 1);

    for i in 0..=POINTS {
        let t = total_time * (i as f32 / POINTS as f32);
        let x = graph.min.x + graph.width() * (i as f32 / POINTS as f32);
        let mut filt = (-decay_curve * t / filter_env_decay.max(0.001)).exp();
        filt = filt.clamp(0.0, 1.0);
        let y = graph.max.y - graph.height() * filt;
        points.push(Pos2::new(x, y));
    }

    if !points.is_empty() {
        painter.add(Shape::line(
            points,
            Stroke::new(2.5, Color32::from_rgb(255, 160, 60)),
        ));
    }

    response
}
