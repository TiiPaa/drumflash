//! Interactive envelope visualizer for the Sound Panel.
//!
//! Two separate views:
//! - `draw_amp_envelope`   : AHDSR-style amplitude curve (yellow/cyan/blue/purple)
//! - `draw_filter_envelope`: Filter envelope curve (orange)
//!
//! The engine is bi-stage parallel (DecayReleaseEnvelope), but the amplitude
//! graph is drawn as a single continuous curve with colour-coded phases so it
//! reads like a standard synth envelope.  This makes it easy to later switch
//! individual voices to a true sequential AHDSR without changing the UI.

use nih_plug_egui::egui::{Color32, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Attack,
    Hold,
    Decay,
    Release,
}

impl Phase {
    fn color(&self) -> Color32 {
        match self {
            Phase::Attack => Color32::from_rgb(255, 220, 80),   // yellow
            Phase::Hold   => Color32::from_rgb(140, 220, 255), // cyan
            Phase::Decay  => Color32::from_rgb(100, 180, 255), // blue
            Phase::Release=> Color32::from_rgb(180, 120, 255), // purple
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Phase::Attack  => "A",
            Phase::Hold    => "H",
            Phase::Decay   => "D",
            Phase::Release => "R",
        }
    }
}

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
    let desired_size = Vec2::new(140.0, 90.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_gray(28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(80)), StrokeKind::Inside);

    // Inner padding so labels don't touch the border
    let pad_x = 14.0f32;
    let pad_y = 10.0f32;
    let graph = Rect::from_min_size(
        rect.min + Vec2::new(pad_x, pad_y),
        rect.size() - Vec2::new(pad_x * 2.0, pad_y * 2.0),
    );

    let total_time = if has_release {
        (attack_time + hold + decay + release).max(0.1)
    } else {
        (attack_time + hold + decay).max(0.1)
    };

    const POINTS: usize = 300;
    let mut segments: Vec<(Phase, Vec<Pos2>)> = Vec::new();
    let mut current_seg: Vec<Pos2> = Vec::new();
    let mut current_phase = Phase::Attack;

    let mut first_label_pos: [Option<Pos2>; 4] = [None; 4];

    let has_attack = attack_time > 0.0;

    for i in 0..=POINTS {
        let t = total_time * (i as f32 / POINTS as f32);
        let x = graph.min.x + graph.width() * (i as f32 / POINTS as f32);

        let phase = if has_attack && t < attack_time {
            Phase::Attack
        } else if t < attack_time + hold {
            Phase::Hold
        } else {
            let t_decay = t - attack_time - hold;
            let d = (-decay_curve * t_decay / decay.max(0.001)).exp();
            let r = if has_release {
                0.3 * (-release_curve * t / release.max(0.001)).exp()
            } else {
                0.0
            };
            if d >= r { Phase::Decay } else { Phase::Release }
        };

        let amp = match phase {
            Phase::Attack => t / attack_time,
            Phase::Hold => 1.0,
            Phase::Decay => {
                let td = (t - attack_time - hold).max(0.0);
                (-decay_curve * td / decay.max(0.001)).exp()
            }
            Phase::Release => {
                if has_release {
                    0.3 * (-release_curve * t / release.max(0.001)).exp()
                } else {
                    0.0
                }
            }
        }.clamp(0.0, 1.0);

        let y = graph.max.y - graph.height() * amp;
        let pos = Pos2::new(x, y);

        let idx = phase as usize;
        if first_label_pos[idx].is_none() {
            first_label_pos[idx] = Some(pos);
        }

        if phase != current_phase {
            if !current_seg.is_empty() {
                segments.push((current_phase, current_seg));
            }
            current_seg = vec![pos];
            current_phase = phase;
        } else {
            current_seg.push(pos);
        }
    }
    if !current_seg.is_empty() {
        segments.push((current_phase, current_seg));
    }

    // Draw each coloured segment
    let mut drawn_label = [false; 4];
    for (phase, seg) in &segments {
        if seg.len() < 2 { continue; }
        painter.add(Shape::line(seg.clone(), Stroke::new(2.5, phase.color())));

        let idx = *phase as usize;
        if !drawn_label[idx] {
            if let Some(lp) = first_label_pos[idx] {
                // Clamp label inside the graph rect with a small offset
                let mut lx = lp.x + 3.0;
                let mut ly = lp.y - 6.0;
                lx = lx.clamp(graph.min.x + 2.0, graph.max.x - 10.0);
                ly = ly.clamp(graph.min.y + 10.0, graph.max.y - 2.0);
                painter.text(
                    Pos2::new(lx, ly),
                    nih_plug_egui::egui::Align2::LEFT_BOTTOM,
                    phase.label(),
                    nih_plug_egui::egui::FontId::monospace(10.0),
                    phase.color(),
                );
                drawn_label[idx] = true;
            }
        }
    }

    response
}

// -- Filter envelope ---------------------------------------------------------

pub fn draw_filter_envelope(
    ui: &mut nih_plug_egui::egui::Ui,
    decay_curve: f32,
    filter_env_decay: f32,
) -> nih_plug_egui::egui::Response {
    let desired_size = Vec2::new(140.0, 90.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 4.0, Color32::from_gray(28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_gray(80)), StrokeKind::Inside);

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
        painter.add(Shape::line(points, Stroke::new(2.5, Color32::from_rgb(255, 160, 60))));
    }

    response
}
