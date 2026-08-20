//! Interactive envelope visualizer for the Sound Panel.
//!
//! All graphs are built the same way ([178]): the shared `prep_graph` frame
//! (recessed LCD + padding + faint quarter gridlines) and the shared stage
//! colors — attack = amber, hold = green, decay = blue. Every A-H-D graph
//! (amp, filter, sample) splits its curve into those colored stages.

use crate::ui::theme::*;
use nih_plug_egui::egui::{Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};

// -- Shared frame & stage colors ([178]) --------------------------------------

/// Full-height graphs (amp / filter / sample) share this height; the Buzz gate
/// strip is the only smaller one.
const GRAPH_H: f32 = 104.0;
const GATE_GRAPH_H: f32 = 72.0;
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 10.0;
const CURVE_W: f32 = 2.0;

/// Attack stage color (amber) — same for every graph.
fn stage_attack() -> Color32 {
    AMBER()
}

/// Hold stage color (green) — same for every graph.
fn stage_hold() -> Color32 {
    Color32::from_rgb(110, 200, 165)
}

/// Decay stage color (blue) — same for every graph; also the color of
/// single-stage (pure exponential decay) curves.
fn stage_decay() -> Color32 {
    BLUE()
}

/// Common scaffolding for every Sound-Panel graph: allocates the LCD rect,
/// paints the recessed green screen and returns the inner graph rect +
/// painter. Grid lines stay a separate call so sample graphs can slide their
/// waveform UNDER the grid.
fn prep_graph(
    ui: &mut nih_plug_egui::egui::Ui,
    height: f32,
) -> (Rect, Painter, nih_plug_egui::egui::Response) {
    let w = ui.available_width().max(120.0);
    let desired_size = Vec2::new(w, height);
    let (rect, response) =
        ui.allocate_at_least(desired_size, nih_plug_egui::egui::Sense::hover());
    crate::ui::skeuo::lcd_bg(ui, rect, RADIUS_PAD as f32);
    let graph = Rect::from_min_size(
        rect.min + Vec2::new(PAD_X, PAD_Y),
        rect.size() - Vec2::new(PAD_X * 2.0, PAD_Y * 2.0),
    );
    (graph, ui.painter_at(rect), response)
}

/// Faint vertical quarter gridlines, drawn on every envelope/filter graph.
fn draw_grid_lines(painter: &Painter, graph: &Rect) {
    for i in 0..=4 {
        let x = graph.min.x + graph.width() * i as f32 / 4.0;
        painter.line_segment(
            [
                Pos2::new(x, graph.min.y),
                Pos2::new(x, graph.max.y),
            ],
            Stroke::new(1.0, white_a(9)),
        );
    }
}

/// Resting cutoff line (where a filter sweep returns), shared by all filter
/// graphs.
fn draw_cutoff_line(painter: &Painter, graph: &Rect, y: f32) {
    painter.line_segment(
        [Pos2::new(graph.min.x, y), Pos2::new(graph.max.x, y)],
        Stroke::new(1.0, white_a(90)),
    );
}

// -- Amplitude envelope (A-H-D) ----------------------------------------------

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
    let (graph, painter, response) = prep_graph(ui, GRAPH_H);
    let base_y = graph.max.y;
    let top_y = graph.min.y;

    draw_grid_lines(&painter, &graph);

    let attack = attack_time.max(0.001);
    let hold_time = hold.max(0.0);
    let decay_time = decay.max(0.02);
    let total_time = attack + hold_time + decay_time;

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
        atk_pts.push(Pos2::new(x, y_of(bipolar_shape_curve(p, atk_curve))));
    }
    painter.add(Shape::line(atk_pts, Stroke::new(CURVE_W, stage_attack())));

    // Hold: flat plateau at peak (only when a hold is set).
    if hold_time > 0.0 {
        painter.line_segment(
            [Pos2::new(x_attack, top_y), Pos2::new(x_hold, top_y)],
            Stroke::new(CURVE_W, stage_hold()),
        );
    }

    // Decay: shaped ramp 1 -> 0 (bipolar decay curve), runs to the baseline.
    let mut dec_pts = Vec::with_capacity(SEG + 1);
    for i in 0..=SEG {
        let p = i as f32 / SEG as f32;
        let x = x_hold + (x_end - x_hold) * p;
        dec_pts.push(Pos2::new(x, y_of(bipolar_shape_curve(1.0 - p, dec_curve))));
    }
    painter.add(Shape::line(dec_pts, Stroke::new(CURVE_W, stage_decay())));

    response
}

// -- Filter envelope ---------------------------------------------------------

/// Filter graph for the synth voices (Toms): the actual CUTOFF SWEEP on a log
/// Hz axis — resting cutoff line + swept curve `cutoff × (1 + env×amount×4)`
/// over a FIXED 1 s window (the decay slider visibly stretches the sweep).
/// With amount = 0 the curve is a flat line at the cutoff: the graph then
/// truthfully says "the envelope does nothing".
pub fn draw_filter_envelope(
    ui: &mut nih_plug_egui::egui::Ui,
    curve: f32,
    filter_env_decay: f32,
    cutoff_hz: f32,
    env_amount: f32,
) -> nih_plug_egui::egui::Response {
    let (graph, painter, response) = prep_graph(ui, GRAPH_H);

    let hz_to_y = |hz: f32| -> f32 {
        let norm = ((hz.max(20.0).min(20000.0)).ln() - 20f32.ln()) / (20000f32.ln() - 20f32.ln());
        graph.max.y - graph.height() * norm.clamp(0.0, 1.0)
    };

    draw_grid_lines(&painter, &graph);

    const SPAN_SECS: f32 = 1.0;
    let c = curve.max(0.1);
    let decay = filter_env_decay.max(0.001);
    let cutoff = cutoff_hz.max(20.0).min(20000.0);
    let amount = env_amount.clamp(0.0, 1.0);
    const POINTS: usize = 200;
    let mut points: Vec<Pos2> = Vec::with_capacity(POINTS + 1);

    for i in 0..=POINTS {
        let p = i as f32 / POINTS as f32;
        let env = (-c * (p * SPAN_SECS) / decay).exp().clamp(0.0, 1.0);
        // Same law as the voice DSP: exponential sweep from the base cutoff
        // toward 20 kHz — `cutoff × (20000/cutoff)^(env×amount)`.
        let hz = cutoff * (20000.0 / cutoff).powf(env * amount);
        let x = graph.min.x + graph.width() * p;
        points.push(Pos2::new(x, hz_to_y(hz)));
    }

    // Single-stage curve: decay color, like every other graph ([178]).
    if !points.is_empty() {
        painter.add(Shape::line(points, Stroke::new(CURVE_W, stage_decay())));
    }

    draw_cutoff_line(&painter, &graph, hz_to_y(cutoff));

    response
}

// -- A-H-D filter envelope (Buzz / SDrex) -------------------------------------

/// Bipolar curve shaping, mirroring `BuzzVoice::shape_curve` / `dsp::shape_curve`
/// (exponent 1+5|c|, [170]).
fn bipolar_shape_curve(e: f32, curve: f32) -> f32 {
    let e = e.clamp(0.0, 1.0);
    let c = curve.clamp(-1.0, 1.0);
    if c >= 0.0 {
        e.powf(1.0 + c * 5.0)
    } else {
        1.0 - (1.0 - e).powf(1.0 - c * 5.0)
    }
}

/// Filter graph for the Buzz/SDrex voices: draws the A-H-D cutoff sweep
/// exactly like the DSP — attack ramp, hold, decay (each with its bipolar
/// curve), the cutoff swept EXPONENTIALLY from the base toward fully open by
/// `env × amount`. Stages use the shared colors ([178]).
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
    let (graph, painter, response) = prep_graph(ui, GRAPH_H);

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
    let y_of_env = |env: f32| -> f32 {
        let amt = (env * amount).clamp(0.0, 1.0);
        hz_to_y(base * (20000.0 / base).powf(amt))
    };
    let x_of_t = |t: f32| graph.min.x + graph.width() * (t / span).clamp(0.0, 1.0);

    draw_grid_lines(&painter, &graph);

    const POINTS: usize = 80;

    // Attack: shaped ramp of the envelope 0 -> 1.
    let mut atk_pts = Vec::with_capacity(POINTS + 1);
    for i in 0..=POINTS {
        let t = attack * (i as f32 / POINTS as f32);
        atk_pts.push(Pos2::new(
            x_of_t(t),
            y_of_env(bipolar_shape_curve(t / attack, atk_curve)),
        ));
    }
    painter.add(Shape::line(atk_pts, Stroke::new(CURVE_W, stage_attack())));

    // Hold: envelope pinned at 1.
    if hold > 0.0 {
        painter.line_segment(
            [Pos2::new(x_of_t(attack), y_of_env(1.0)), Pos2::new(x_of_t(attack + hold), y_of_env(1.0))],
            Stroke::new(CURVE_W, stage_hold()),
        );
    }

    // Decay: shaped ramp 1 -> 0, drawn over the rest of the window so it
    // visibly lands on the resting cutoff.
    let mut dec_pts = Vec::with_capacity(POINTS + 2);
    for i in 0..=POINTS {
        let t = (attack + hold) + (span - attack - hold) * (i as f32 / POINTS as f32);
        let p = ((t - attack - hold) / decay).clamp(0.0, 1.0);
        dec_pts.push(Pos2::new(x_of_t(t), y_of_env(bipolar_shape_curve(1.0 - p, dec_curve))));
    }
    painter.add(Shape::line(dec_pts, Stroke::new(CURVE_W, stage_decay())));

    draw_cutoff_line(&painter, &graph, hz_to_y(base));

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
    let (graph, painter, response) = prep_graph(ui, GATE_GRAPH_H);

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
    draw_grid_lines(&painter, &graph);
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
    painter.add(Shape::line(points, Stroke::new(CURVE_W, stage_decay())));

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

/// Amp graph for multisample voices: cropped waveform + amp envelope.
/// [174/F3] A-H-D bipolar, faithful to the DSP (`DecayReleaseEnvelope`):
/// shaped attack (absolute, ≤80 ms) then shaped decay. The x split between
/// attack and decay mixes units (seconds vs fraction of the region), so the
/// proportion is approximate — but both curve SHAPES mirror the engine.
#[allow(clippy::too_many_arguments)]
pub fn draw_sample_amp_graph(
    ui: &mut nih_plug_egui::egui::Ui,
    hit: &[f32],
    start_frac: f32,
    end_frac: f32,
    attack_frac: f32,
    decay_frac: f32,
    attack_curve: f32,
    decay_curve: f32,
    one_shot: bool,
) -> nih_plug_egui::egui::Response {
    let (graph, painter, response) = prep_graph(ui, GRAPH_H);
    let start = start_frac.clamp(0.0, 1.0);
    let end = end_frac.clamp(0.0, 1.0).max(start + 0.01);
    draw_waveform(&painter, &graph, hit, start, end);

    let top_y = graph.min.y;
    let base_y = graph.max.y;
    draw_grid_lines(&painter, &graph);

    if one_shot {
        // Amp envelope is bypassed: flat full-level line, greyed out.
        painter.line_segment(
            [Pos2::new(graph.min.x, top_y), Pos2::new(graph.max.x, top_y)],
            Stroke::new(CURVE_W, white_a(60)),
        );
    } else {
        let attack_s = attack_frac.clamp(0.0, 1.0) * 0.08; // MAX_AMP_ATTACK_SECS
        let decay_rel = decay_frac.clamp(0.01, 1.0);
        let total = attack_s + decay_rel;
        let p_a = (attack_s / total).clamp(0.0, 1.0);
        let y_of = |v: f32| base_y - (base_y - top_y) * v.clamp(0.0, 1.0);
        let x_of = |p: f32| graph.min.x + graph.width() * p;

        const POINTS: usize = 40;

        // Attack stage (amber), shaped ramp 0 -> 1.
        let mut atk_pts = Vec::with_capacity(POINTS + 1);
        for i in 0..=POINTS {
            let p = p_a * (i as f32 / POINTS as f32);
            let v = bipolar_shape_curve(p / p_a.max(1e-6), attack_curve);
            atk_pts.push(Pos2::new(x_of(p), y_of(v)));
        }
        painter.add(Shape::line(atk_pts, Stroke::new(CURVE_W, stage_attack())));

        // Decay stage (blue), shaped ramp 1 -> 0.
        let mut dec_pts = Vec::with_capacity(POINTS + 1);
        for i in 0..=POINTS {
            let p = p_a + (1.0 - p_a) * (i as f32 / POINTS as f32);
            let v = bipolar_shape_curve(1.0 - (p - p_a) / (1.0 - p_a).max(1e-6), decay_curve);
            dec_pts.push(Pos2::new(x_of(p), y_of(v)));
        }
        painter.add(Shape::line(dec_pts, Stroke::new(CURVE_W, stage_decay())));
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
    let (graph, painter, response) = prep_graph(ui, GRAPH_H);
    let start = start_frac.clamp(0.0, 1.0);
    let end = end_frac.clamp(0.0, 1.0).max(start + 0.01);
    let region_len = end - start;
    draw_waveform(&painter, &graph, hit, start, end);

    let hz_to_y = |hz: f32| -> f32 {
        let norm = ((hz.max(20.0).min(20000.0)).ln() - 20f32.ln()) / (20000f32.ln() - 20f32.ln());
        graph.max.y - graph.height() * norm.clamp(0.0, 1.0)
    };

    draw_grid_lines(&painter, &graph);

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
    // Single-stage curve: decay color, like every other graph ([178]).
    if points.len() > 1 {
        painter.add(Shape::line(points, Stroke::new(CURVE_W, stage_decay())));
    }

    // Resting cutoff line across the whole graph.
    draw_cutoff_line(&painter, &graph, hz_to_y(cutoff_hz));

    response
}
