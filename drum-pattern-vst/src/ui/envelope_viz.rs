//! Interactive envelope visualizer for the Sound Panel.
//!
//! Two separate views:
//! - `draw_amp_envelope`   : ADSR-style amplitude curve (amber/blue/purple)
//! - `draw_filter_envelope`: Filter envelope curve (orange)
//!
//! The engine is bi-stage parallel (DecayReleaseEnvelope), but the amplitude
//! graph intentionally follows the designer mockup's simplified ADSR readout.

use crate::ui::theme::*;
use nih_plug_egui::egui::{Align2, Color32, Pos2, Rect, Shape, Stroke, Vec2};

// -- Amplitude envelope (AHDSR style) ----------------------------------------

/// Amplitude envelope readout: **A-H-D** with independent BIPOLAR curve shaping
/// on the attack and the decay (no release stage), mirroring the DSP.
pub fn draw_amp_envelope(
    ui: &mut nih_plug_egui::egui::Ui,
    attack_time: f32,
    atk_curve: f32,
    hold: f32,
    decay: f32,
    dec_curve: f32,
) -> nih_plug_egui::egui::Response {
    let w = ui.available_width().max(120.0);
    let desired_size = Vec2::new(w, 104.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Recessed green LCD screen (one place: `skeuo::lcd_bg`); curve drawn on top.
    crate::ui::skeuo::lcd_bg(ui, rect, RADIUS_PAD as f32);

    let pad_x = 12.0f32;
    let pad_y = 12.0f32;
    let graph = Rect::from_min_size(
        rect.min + Vec2::new(pad_x, pad_y),
        rect.size() - Vec2::new(pad_x * 2.0, pad_y * 2.0),
    );
    // Reserve a bottom strip for the A/H/D legend (letters no longer on the curve).
    let legend_h = 14.0f32;
    let base_y = graph.max.y - legend_h;
    let top_y = graph.min.y;

    for i in 0..=4 {
        let x = graph.min.x + graph.width() * i as f32 / 4.0;
        painter.line_segment(
            [Pos2::new(x, top_y), Pos2::new(x, base_y)],
            Stroke::new(1.0, white_a(13)),
        );
    }

    let attack = attack_time.max(0.001);
    let hold_time = hold.max(0.0);
    let decay_time = decay.max(0.02);
    let total_time = attack + hold_time + decay_time;

    let hold_col = Color32::from_rgb(110, 200, 165);
    let x_start = graph.min.x;
    let x_attack = x_start + graph.width() * attack / total_time;
    let x_hold = x_attack + graph.width() * hold_time / total_time;
    let x_end = graph.max.x;

    let y_of = |v: f32| base_y - (base_y - top_y) * v.clamp(0.0, 1.0);
    const SEG: usize = 32;

    // Attack: shaped ramp 0 -> 1 (bipolar attack curve).
    let mut atk_pts = Vec::with_capacity(SEG + 1);
    for i in 0..=SEG {
        let p = i as f32 / SEG as f32;
        let x = x_start + (x_attack - x_start) * p;
        atk_pts.push(Pos2::new(x, y_of(buzz_shape_curve(p, atk_curve))));
    }
    painter.add(Shape::line(atk_pts, Stroke::new(2.0, AMBER())));

    // Hold: flat plateau at peak (only when a hold is set).
    if hold_time > 0.0 {
        painter.line_segment(
            [Pos2::new(x_attack, top_y), Pos2::new(x_hold, top_y)],
            Stroke::new(2.0, hold_col),
        );
    }

    // Decay: shaped ramp 1 -> 0 (bipolar decay curve), runs to the baseline.
    let mut dec_pts = Vec::with_capacity(SEG + 1);
    for i in 0..=SEG {
        let p = i as f32 / SEG as f32;
        let x = x_hold + (x_end - x_hold) * p;
        dec_pts.push(Pos2::new(x, y_of(buzz_shape_curve(1.0 - p, dec_curve))));
    }
    painter.add(Shape::line(dec_pts, Stroke::new(2.0, BLUE())));

    // Bottom legend (coloured square + letter) - no letters on the curve itself.
    let mut items: Vec<(Color32, &str)> = vec![(AMBER(), "A")];
    if hold_time > 0.0 {
        items.push((hold_col, "H"));
    }
    items.push((BLUE(), "D"));
    let ly = graph.max.y - legend_h * 0.5;
    let mut lx = graph.min.x + 2.0;
    for (col, letter) in &items {
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(lx, ly - 4.0), Vec2::splat(8.0)),
            1.0,
            *col,
        );
        painter.text(
            Pos2::new(lx + 12.0, ly),
            nih_plug_egui::egui::Align2::LEFT_CENTER,
            *letter,
            f_mono_med(9.0),
            white_a(150),
        );
        lx += 40.0;
    }

    response
}

// -- Filter envelope ---------------------------------------------------------

pub fn draw_filter_envelope(
    ui: &mut nih_plug_egui::egui::Ui,
    curve: f32,
    filter_env_decay: f32,
) -> nih_plug_egui::egui::Response {
    let w = ui.available_width().max(120.0);
    let desired_size = Vec2::new(w, 104.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Recessed green LCD screen (one place: `skeuo::lcd_bg`); curve drawn on top.
    crate::ui::skeuo::lcd_bg(ui, rect, RADIUS_PAD as f32);

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
        let mut filt = (-curve * t / filter_env_decay.max(0.001)).exp();
        filt = filt.clamp(0.0, 1.0);
        let y = graph.max.y - graph.height() * filt;
        points.push(Pos2::new(x, y));
    }

    if !points.is_empty() {
        painter.add(Shape::line(points, Stroke::new(2.5, ENVELOPE_CURVE())));
    }

    response
}

// -- Buzz A-H-D filter envelope ----------------------------------------------

/// Bipolar curve shaping, mirroring `BuzzVoice::shape_curve`.
fn buzz_shape_curve(e: f32, curve: f32) -> f32 {
    let e = e.clamp(0.0, 1.0);
    let c = curve.clamp(-1.0, 1.0);
    if c >= 0.0 {
        e.powf(1.0 + c * 3.0)
    } else {
        1.0 - (1.0 - e).powf(1.0 - c * 3.0)
    }
}

/// Filter graph for the Buzz voice: draws the A-H-D cutoff sweep exactly like
/// the DSP — attack ramp, hold, decay (with the bipolar curve), the cutoff
/// swept EXPONENTIALLY from the base toward fully open by `env × amount`.
#[allow(clippy::too_many_arguments)]
pub fn draw_buzz_filter_envelope(
    ui: &mut nih_plug_egui::egui::Ui,
    base_cutoff_hz: f32,
    env_amount: f32,
    attack: f32,
    hold: f32,
    decay: f32,
    atk_curve: f32,
    dec_curve: f32,
) -> nih_plug_egui::egui::Response {
    let w = ui.available_width().max(120.0);
    let desired_size = Vec2::new(w, 104.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    crate::ui::skeuo::lcd_bg(ui, rect, RADIUS_PAD as f32);

    let pad_x = 14.0f32;
    let pad_y = 10.0f32;
    let graph = Rect::from_min_size(
        rect.min + Vec2::new(pad_x, pad_y),
        rect.size() - Vec2::new(pad_x * 2.0, pad_y * 2.0),
    );

    let base = base_cutoff_hz.max(20.0).min(20000.0);
    let amount = env_amount.clamp(0.0, 1.0);
    let attack = attack.max(0.0005);
    let hold = hold.max(0.0);
    let decay = decay.max(0.01);
    // Show the full A-H-D plus a short tail so the decay lands on the baseline.
    let span = (attack + hold + decay) * 1.15;

    let hz_to_y = |hz: f32| -> f32 {
        let norm = ((hz.max(20.0).min(20000.0)).ln() - 20f32.ln()) / (20000f32.ln() - 20f32.ln());
        graph.max.y - graph.height() * norm.clamp(0.0, 1.0)
    };

    const POINTS: usize = 200;
    let mut points: Vec<Pos2> = Vec::with_capacity(POINTS + 1);
    for i in 0..=POINTS {
        let t = span * (i as f32 / POINTS as f32);
        // Same A-H-D shape as the DSP: linear attack & decay ramps, each shaped
        // by its own bipolar curve.
        let env = if t < attack {
            buzz_shape_curve(t / attack, atk_curve)
        } else if t < attack + hold {
            1.0
        } else {
            let p = ((t - attack - hold) / decay).clamp(0.0, 1.0);
            buzz_shape_curve(1.0 - p, dec_curve)
        };
        let amt = (env * amount).clamp(0.0, 1.0);
        let hz = base * (20000.0 / base).powf(amt);
        let x = graph.min.x + graph.width() * (i as f32 / POINTS as f32);
        points.push(Pos2::new(x, hz_to_y(hz)));
    }
    if points.len() > 1 {
        painter.add(Shape::line(points, Stroke::new(2.5, ENVELOPE_CURVE())));
    }

    // Resting cutoff line (the base the sweep returns to).
    let cutoff_y = hz_to_y(base);
    painter.line_segment(
        [
            Pos2::new(graph.min.x, cutoff_y),
            Pos2::new(graph.max.x, cutoff_y),
        ],
        Stroke::new(1.0, white_a(90)),
    );

    response
}

// -- Buzz gate shape ----------------------------------------------------------

/// Gate shape graph for the Buzz voice: the amplitude gate over a FIXED 60 ms
/// time window (so the Rate is visible: ~3 cycles at the 55 Hz default, a
/// dense comb at 500 Hz), mirroring the DSP — Smooth = raised-cosine tremolo
/// (Shape narrows the pulse), Razor = sharp exponential spike retriggered from
/// zero each cycle. Depth sets the floor: `gate_mod = 1 - depth·(1 - g)`.
pub fn draw_buzz_gate_graph(
    ui: &mut nih_plug_egui::egui::Ui,
    razor: bool,
    rate: f32,
    depth: f32,
    shape: f32,
) -> nih_plug_egui::egui::Response {
    let w = ui.available_width().max(120.0);
    let desired_size = Vec2::new(w, 72.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    let painter = ui.painter_at(rect);
    crate::ui::skeuo::lcd_bg(ui, rect, RADIUS_PAD as f32);

    let pad = 8.0f32;
    let graph = Rect::from_min_size(
        rect.min + Vec2::new(pad, pad),
        rect.size() - Vec2::new(pad * 2.0, pad * 2.0),
    );

    let rate = rate.clamp(1.0, 500.0);
    let depth = depth.clamp(0.0, 1.0);
    let shape = shape.clamp(0.0, 1.0);
    let period = 1.0 / rate;
    let span = 0.06f32; // fixed time window (seconds)

    // Razor spike: same constants as `BuzzVoice` (0.3 ms attack ramp, decay
    // length + curve relative to the gate period).
    let razor_attack = 0.0003f32;
    let razor_decay = (period * 0.8 * 0.05f32.powf(shape)).max(0.0002);
    let razor_curve = 3.0 + 9.0 * shape;

    let top_y = graph.min.y;
    let base_y = graph.max.y;
    const POINTS: usize = 240;
    let mut points: Vec<Pos2> = Vec::with_capacity(POINTS + 1);
    for i in 0..=POINTS {
        let t = span * (i as f32 / POINTS as f32);
        let g = if razor {
            let tc = t % period;
            if tc < razor_attack {
                tc / razor_attack
            } else {
                (-razor_curve * (tc - razor_attack) / razor_decay).exp()
            }
        } else {
            let raw = 0.5 + 0.5 * (t / period * std::f32::consts::TAU).cos();
            raw.powf(1.0 + shape * 4.0)
        };
        let gate_mod = 1.0 - depth * (1.0 - g);
        let x = graph.min.x + graph.width() * (i as f32 / POINTS as f32);
        let y = base_y - (base_y - top_y) * gate_mod.clamp(0.0, 1.0);
        points.push(Pos2::new(x, y));
    }
    painter.add(Shape::line(points, Stroke::new(2.0, BLUE())));

    // Depth floor: the level the gate chops down to.
    if depth > 0.001 && depth < 0.999 {
        let floor_y = base_y - (base_y - top_y) * (1.0 - depth);
        painter.line_segment(
            [
                Pos2::new(graph.min.x, floor_y),
                Pos2::new(graph.max.x, floor_y),
            ],
            Stroke::new(1.0, white_a(50)),
        );
    }

    // Corner tag so the two stacked LCDs (amp env above) stay identifiable.
    painter.text(
        Pos2::new(graph.min.x + 2.0, graph.min.y + 1.0),
        Align2::LEFT_TOP,
        "GATE",
        f_sans_sb(9.0),
        white_a(110),
    );

    response
}

// -- Multisample graphs (BD6smp / SD6smp) -------------------------------------

/// Depth of the additive filter envelope, mirrors the voices' constant.
const SMP_FILTER_ENV_DEPTH_HZ: f32 = 8000.0;

fn prep_graph(
    ui: &mut nih_plug_egui::egui::Ui,
) -> (
    Rect,
    nih_plug_egui::egui::Painter,
    nih_plug_egui::egui::Response,
) {
    let w = ui.available_width().max(120.0);
    let desired_size = Vec2::new(w, 104.0);
    let (rect, response) = ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    crate::ui::skeuo::lcd_bg(ui, rect, RADIUS_PAD as f32);
    let pad = 8.0f32;
    let graph = Rect::from_min_size(
        rect.min + Vec2::new(pad, pad),
        rect.size() - Vec2::new(pad * 2.0, pad * 2.0),
    );
    (graph, ui.painter_at(rect), response)
}

/// Waveform of the played region [start, end], cropped (the offset parts are
/// NOT drawn), normalised, centred - wide stylised bars, dim under the curve.
fn draw_waveform(
    painter: &nih_plug_egui::egui::Painter,
    graph: &Rect,
    hit: &[f32],
    start_frac: f32,
    end_frac: f32,
) {
    const COLS: usize = 64;
    if hit.len() < 2 {
        return;
    }
    let first = (start_frac.clamp(0.0, 1.0) * hit.len() as f32) as usize;
    let last = (end_frac.clamp(0.0, 1.0) * hit.len() as f32) as usize;
    let last = last.max(first + 2).min(hit.len());
    let region = &hit[first..last];

    let center_y = graph.min.y + graph.height() * 0.5;
    let half_h = graph.height() * 0.48;
    let global_peak = region
        .iter()
        .map(|s| s.abs())
        .fold(0.0f32, f32::max)
        .max(1e-6);
    let col_w = graph.width() / COLS as f32;
    for c in 0..COLS {
        let start = c * region.len() / COLS;
        let end = ((c + 1) * region.len() / COLS).max(start + 1);
        let peak = region[start..end.min(region.len())]
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        let x = graph.min.x + col_w * (c as f32 + 0.5);
        let h = (peak / global_peak * half_h).max(0.75);
        painter.line_segment(
            [Pos2::new(x, center_y - h), Pos2::new(x, center_y + h)],
            Stroke::new(2.0, white_a(70)),
        );
    }
}

/// Maps an envelope time (fraction of the FULL sample length) to the x axis,
/// which spans the played region [start, end].
fn env_x(graph: &Rect, region_len: f32, t: f32) -> f32 {
    graph.min.x + graph.width() * (t / region_len).clamp(0.0, 1.0)
}

/// Amp graph for multisample voices: cropped waveform + amp envelope
/// (attack & decay are fractions of the FULL played sample length).
pub fn draw_sample_amp_graph(
    ui: &mut nih_plug_egui::egui::Ui,
    hit: &[f32],
    start_frac: f32,
    end_frac: f32,
    attack_frac: f32,
    decay_frac: f32,
    decay_curve: f32,
    one_shot: bool,
) -> nih_plug_egui::egui::Response {
    let (graph, painter, response) = prep_graph(ui);
    let start = start_frac.clamp(0.0, 1.0);
    let end = end_frac.clamp(0.0, 1.0).max(start + 0.01);
    let region_len = end - start;
    draw_waveform(&painter, &graph, hit, start, end);

    let top_y = graph.min.y;
    let base_y = graph.max.y;
    let attack = attack_frac.clamp(0.0, 1.0);
    let decay = decay_frac.clamp(0.01, 1.0);

    if one_shot {
        // Amp envelope is bypassed: flat full-level line, greyed out.
        painter.line_segment(
            [Pos2::new(graph.min.x, top_y), Pos2::new(graph.max.x, top_y)],
            Stroke::new(2.0, white_a(60)),
        );
    } else {
        const POINTS: usize = 80;
        let mut points: Vec<Pos2> = Vec::with_capacity(POINTS + 2);
        for i in 0..=POINTS {
            let t = i as f32 / POINTS as f32; // env time, fraction of the hit length
            let v = if t < attack {
                if attack > 0.0 { t / attack } else { 1.0 }
            } else {
                (-decay_curve.max(0.1) * (t - attack) / decay).exp()
            };
            let x = env_x(&graph, region_len, t);
            let y = base_y - (base_y - top_y) * v.clamp(0.0, 1.0);
            points.push(Pos2::new(x, y));
            if t >= region_len {
                break;
            }
        }
        // Make sure the curve visibly lands on the baseline at the region edge.
        if let Some(last) = points.last().copied() {
            if last.x < graph.max.x - 1.0 && last.y < base_y - 1.0 {
                points.push(Pos2::new(graph.max.x, base_y));
            }
        }
        if points.len() > 1 {
            painter.add(Shape::line(points, Stroke::new(2.0, BLUE())));
        }
    }

    response
}

/// Filter graph for multisample voices: cropped waveform + cutoff line +
/// filter envelope sweep, held at the cutoff after the sweep so the end of
/// the curve stays visible (decay is a fraction of the FULL sample length).
pub fn draw_sample_filter_graph(
    ui: &mut nih_plug_egui::egui::Ui,
    hit: &[f32],
    start_frac: f32,
    end_frac: f32,
    cutoff_hz: f32,
    env_amount: f32,
    env_decay_frac: f32,
    curve: f32,
) -> nih_plug_egui::egui::Response {
    let (graph, painter, response) = prep_graph(ui);
    let start = start_frac.clamp(0.0, 1.0);
    let end = end_frac.clamp(0.0, 1.0).max(start + 0.01);
    let region_len = end - start;
    draw_waveform(&painter, &graph, hit, start, end);

    let hz_to_y = |hz: f32| -> f32 {
        let norm = ((hz.max(20.0).min(20000.0)).ln() - 20f32.ln()) / (20000f32.ln() - 20f32.ln());
        graph.max.y - graph.height() * norm.clamp(0.0, 1.0)
    };

    // Envelope sweep from the playback start, then held at the resting cutoff.
    let decay = env_decay_frac.clamp(0.01, 1.0);
    let amount = env_amount.clamp(0.0, 1.0);
    const POINTS: usize = 80;
    let mut points: Vec<Pos2> = Vec::with_capacity(POINTS + 2);
    for i in 0..=POINTS {
        let t = decay * (i as f32 / POINTS as f32);
        let env = (-curve.max(0.1) * t / decay).exp();
        let hz = cutoff_hz + env * amount * SMP_FILTER_ENV_DEPTH_HZ;
        points.push(Pos2::new(env_x(&graph, region_len, t), hz_to_y(hz)));
        if t >= region_len {
            break;
        }
    }
    // Hold the tail at the resting cutoff so the curve's end is visible.
    if let Some(last) = points.last().copied() {
        if last.x < graph.max.x - 1.0 {
            points.push(Pos2::new(graph.max.x, hz_to_y(cutoff_hz)));
        }
    }
    if points.len() > 1 {
        painter.add(Shape::line(points, Stroke::new(2.0, ENVELOPE_CURVE())));
    }

    // Resting cutoff line across the whole graph.
    let cutoff_y = hz_to_y(cutoff_hz);
    painter.line_segment(
        [
            Pos2::new(graph.min.x, cutoff_y),
            Pos2::new(graph.max.x, cutoff_y),
        ],
        Stroke::new(1.0, white_a(90)),
    );

    response
}
