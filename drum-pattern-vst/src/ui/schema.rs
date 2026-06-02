/**
 * Flash Drum — Schema data-driven pour l'éditeur de son
 *
 * Source de vérité : assets/fd-data.js
 * L'éditeur ne contient AUCUN paramètre codé en dur.
 * Il parcourt ce schéma pour se reconstruire dynamiquement.
 */

use std::sync::LazyLock;

// ============================================================
// Types de contrôles
// ============================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CtlKind {
    Slider,
    Select,
    Switch,
}

// ============================================================
// Spécification d'un paramètre
// ============================================================

#[derive(Clone, Debug)]
pub struct ParamSpec {
    pub label: &'static str,
    pub key: &'static str,
    pub kind: CtlKind,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub default: f32,
    pub unit: &'static str,
    pub options: &'static [&'static str],
    pub log_scale: bool,
}

// ============================================================
// Section d'éditeur
// ============================================================

#[derive(Clone, Debug)]
pub struct Section {
    pub title: &'static str,
    pub cols: u8,
    pub items: Vec<ParamSpec>,
    pub has_adsr: bool,
}

// ============================================================
// Catégories d'instruments
// ============================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Kick,
    Tom,
    Snare,
    Hat,
    Cymbal,
    Clap,
    Perc,
}

impl Category {
    pub fn schema(self) -> &'static [Section] {
        match self {
            Category::Kick => &KICK_SCHEMA,
            Category::Tom => &TOM_SCHEMA,
            Category::Snare => &SNARE_SCHEMA,
            Category::Hat => &HAT_SCHEMA,
            Category::Cymbal => &CYMBAL_SCHEMA,
            Category::Clap => &CLAP_SCHEMA,
            Category::Perc => &PERC_SCHEMA,
        }
    }

    pub fn filter_label(self) -> &'static str {
        match self {
            Category::Hat | Category::Cymbal => "Filter (HP)",
            _ => "Filter (LP)",
        }
    }
}

// ============================================================
// Helpers de construction
// ============================================================

fn slider(label: &'static str, key: &'static str, min: f32, max: f32, step: f32, default: f32, unit: &'static str) -> ParamSpec {
    ParamSpec { label, key, kind: CtlKind::Slider, min, max, step, default, unit, options: &[], log_scale: false }
}

fn slider_log(label: &'static str, key: &'static str, min: f32, max: f32, step: f32, default: f32, unit: &'static str) -> ParamSpec {
    ParamSpec { label, key, kind: CtlKind::Slider, min, max, step, default, unit, options: &[], log_scale: true }
}

fn select(label: &'static str, key: &'static str, options: &'static [&'static str], default_idx: usize) -> ParamSpec {
    ParamSpec { label, key, kind: CtlKind::Select, min: 0.0, max: (options.len().saturating_sub(1)) as f32, step: 1.0, default: default_idx as f32, unit: "", options, log_scale: false }
}

fn switch(label: &'static str, key: &'static str, default: bool) -> ParamSpec {
    ParamSpec { label, key, kind: CtlKind::Switch, min: 0.0, max: 1.0, step: 1.0, default: if default { 1.0 } else { 0.0 }, unit: "", options: &[], log_scale: false }
}

fn section(title: &'static str, cols: u8, items: Vec<ParamSpec>) -> Section {
    Section { title, cols, items, has_adsr: false }
}

fn section_adsr(title: &'static str, cols: u8, items: Vec<ParamSpec>) -> Section {
    Section { title, cols, items, has_adsr: true }
}

// ============================================================
// Paramètres standards
// ============================================================

fn std_volume() -> ParamSpec { slider("Volume", "volume", 0.0, 2.0, 0.01, 0.8, "") }
fn std_pan() -> ParamSpec { slider("Pan", "pan", -1.0, 1.0, 0.01, 0.0, "") }
fn std_attack() -> ParamSpec { slider("Attack", "attack_ms", 0.0, 500.0, 1.0, 1.5, "ms") }
fn std_decay() -> ParamSpec { slider("Decay", "decay", 0.01, 2.0, 0.01, 0.3, "s") }
fn std_release() -> ParamSpec { slider("Release", "release", 0.01, 2.0, 0.01, 0.2, "s") }
fn std_sustain() -> ParamSpec { slider("Sustain", "sustain", 0.0, 1.0, 0.01, 0.0, "") }
fn std_cutoff() -> ParamSpec { slider_log("Cutoff", "filter_freq", 20.0, 20000.0, 1.0, 8000.0, "Hz") }
fn std_resonance() -> ParamSpec { slider("Resonance", "filter_res", 0.0, 1.0, 0.01, 0.0, "") }
fn std_drive() -> ParamSpec { slider("Drive", "sat_drive", 0.0, 20.0, 0.1, 0.0, "x") }
fn std_sat_type() -> ParamSpec { select("Type", "sat_type", &["SoftClip", "Valve", "Transistor", "HardClip", "Tape"], 0) }
fn std_sat_mix() -> ParamSpec { slider("Mix", "sat_mix", 0.0, 1.0, 0.01, 1.0, "") }
fn std_output_gain() -> ParamSpec { slider("Output Gain", "output_gain", 0.0, 2.0, 0.01, 1.0, "") }
fn std_mix_bus() -> ParamSpec { switch("Mix Bus", "mix_bus", true) }

// ============================================================
// Sections communes (LazyLock)
// ============================================================

static COMMON_LEVEL: LazyLock<Section> = LazyLock::new(|| section("Level", 2, vec![std_volume(), std_pan()]));
static COMMON_ENVELOPE: LazyLock<Section> = LazyLock::new(|| section_adsr("Envelope", 4, vec![std_attack(), std_decay(), std_sustain(), std_release()]));
static COMMON_FILTER: LazyLock<Section> = LazyLock::new(|| section("Filter", 2, vec![std_cutoff(), std_resonance()]));
static COMMON_SATURATION: LazyLock<Section> = LazyLock::new(|| section("Saturation", 3, vec![std_sat_type(), std_drive(), std_sat_mix()]));
static COMMON_OUTPUT: LazyLock<Section> = LazyLock::new(|| section("Output", 2, vec![std_output_gain(), std_mix_bus()]));

// ============================================================
// Sections source
// ============================================================

static SOURCE_KICK: LazyLock<Section> = LazyLock::new(|| section("Oscillator", 3, vec![
    slider("Frequency", "frequency", 20.0, 200.0, 1.0, 60.0, "Hz"),
    slider("Click", "click", 0.0, 1.0, 0.01, 0.5, ""),
    select("Algorithm", "algo", &["Sine", "Triangle", "Saw", "Square"], 0),
]));

static SOURCE_TOM: LazyLock<Section> = LazyLock::new(|| section("Oscillator", 4, vec![
    slider("Frequency", "frequency", 40.0, 400.0, 1.0, 120.0, "Hz"),
    slider("Click", "click", 0.0, 1.0, 0.01, 0.3, ""),
    select("Algorithm", "algo", &["Sine", "Triangle", "Saw", "Square"], 0),
    slider("Pitch Bend", "pitch_bend", 0.0, 1.0, 0.01, 0.2, ""),
]));

static SOURCE_SNARE: LazyLock<Section> = LazyLock::new(|| section("Body + Noise", 4, vec![
    slider("Tone Freq", "tone_freq", 100.0, 1000.0, 1.0, 250.0, "Hz"),
    slider("Noise Mix", "noise_mix", 0.0, 1.0, 0.01, 0.5, ""),
    slider("Snap", "snap", 0.0, 1.0, 0.01, 0.6, ""),
    select("Body", "body_type", &["Synth", "Noise", "Layered"], 0),
]));

static SOURCE_HAT: LazyLock<Section> = LazyLock::new(|| section("Metal / Noise", 4, vec![
    slider("Tone", "tone", 0.0, 1.0, 0.01, 0.5, ""),
    slider("Decay", "hat_decay", 0.01, 1.0, 0.01, 0.15, "s"),
    slider("Color", "color", 0.0, 1.0, 0.01, 0.5, ""),
    slider("Shimmer", "shimmer", 0.0, 1.0, 0.01, 0.0, ""),
]));

static SOURCE_CYMBAL: LazyLock<Section> = LazyLock::new(|| section("Metal / Noise", 4, vec![
    slider("Tone", "tone", 0.0, 1.0, 0.01, 0.5, ""),
    slider("Decay", "cymbal_decay", 0.1, 4.0, 0.01, 1.2, "s"),
    slider("Color", "color", 0.0, 1.0, 0.01, 0.5, ""),
    slider("Shimmer", "shimmer", 0.0, 1.0, 0.01, 0.3, ""),
]));

static SOURCE_CLAP: LazyLock<Section> = LazyLock::new(|| section("Clap Engine", 3, vec![
    slider("Pitch", "clap_pitch", 0.5, 2.0, 0.01, 1.0, ""),
    slider("Spread", "clap_spread", 0.0, 1.0, 0.01, 0.3, ""),
    slider("Count", "clap_count", 1.0, 8.0, 1.0, 3.0, ""),
]));

static SOURCE_PERC: LazyLock<Section> = LazyLock::new(|| section("Source", 3, vec![
    slider_log("Pitch", "pitch", 20.0, 2000.0, 1.0, 440.0, "Hz"),
    slider("Decay", "perc_decay", 0.01, 1.0, 0.01, 0.2, "s"),
    slider("Noise", "noise_amount", 0.0, 1.0, 0.01, 0.0, ""),
]));

// ============================================================
// Assemblage par catégorie
// ============================================================

static KICK_SCHEMA: LazyLock<Vec<Section>> = LazyLock::new(|| vec![
    SOURCE_KICK.clone(),
    COMMON_LEVEL.clone(),
    COMMON_ENVELOPE.clone(),
    COMMON_FILTER.clone(),
    COMMON_SATURATION.clone(),
    COMMON_OUTPUT.clone(),
]);

static TOM_SCHEMA: LazyLock<Vec<Section>> = LazyLock::new(|| vec![
    SOURCE_TOM.clone(),
    COMMON_LEVEL.clone(),
    COMMON_ENVELOPE.clone(),
    COMMON_FILTER.clone(),
    COMMON_SATURATION.clone(),
    COMMON_OUTPUT.clone(),
]);

static SNARE_SCHEMA: LazyLock<Vec<Section>> = LazyLock::new(|| vec![
    SOURCE_SNARE.clone(),
    COMMON_LEVEL.clone(),
    COMMON_ENVELOPE.clone(),
    COMMON_FILTER.clone(),
    COMMON_SATURATION.clone(),
    COMMON_OUTPUT.clone(),
]);

static HAT_SCHEMA: LazyLock<Vec<Section>> = LazyLock::new(|| vec![
    SOURCE_HAT.clone(),
    COMMON_LEVEL.clone(),
    COMMON_ENVELOPE.clone(),
    COMMON_SATURATION.clone(),
    COMMON_OUTPUT.clone(),
]);

static CYMBAL_SCHEMA: LazyLock<Vec<Section>> = LazyLock::new(|| vec![
    SOURCE_CYMBAL.clone(),
    COMMON_LEVEL.clone(),
    COMMON_ENVELOPE.clone(),
    COMMON_SATURATION.clone(),
    COMMON_OUTPUT.clone(),
]);

static CLAP_SCHEMA: LazyLock<Vec<Section>> = LazyLock::new(|| vec![
    SOURCE_CLAP.clone(),
    COMMON_LEVEL.clone(),
    COMMON_ENVELOPE.clone(),
    COMMON_FILTER.clone(),
    COMMON_SATURATION.clone(),
    COMMON_OUTPUT.clone(),
]);

static PERC_SCHEMA: LazyLock<Vec<Section>> = LazyLock::new(|| vec![
    SOURCE_PERC.clone(),
    COMMON_LEVEL.clone(),
    COMMON_ENVELOPE.clone(),
    COMMON_FILTER.clone(),
    COMMON_SATURATION.clone(),
    COMMON_OUTPUT.clone(),
]);

// ============================================================
// API publique — retourne des slices stables
// ============================================================

pub fn schema_for(category: Category) -> &'static [Section] {
    match category {
        Category::Kick => &KICK_SCHEMA,
        Category::Tom => &TOM_SCHEMA,
        Category::Snare => &SNARE_SCHEMA,
        Category::Hat => &HAT_SCHEMA,
        Category::Cymbal => &CYMBAL_SCHEMA,
        Category::Clap => &CLAP_SCHEMA,
        Category::Perc => &PERC_SCHEMA,
    }
}

// ============================================================
// Mapping instrument → catégorie
// ============================================================

pub fn category_for_instrument(idx: usize) -> Category {
    use Category::*;
    match idx {
        0 => Kick,   // BD
        1 => Snare,  // SD
        2 => Hat,    // HH
        3 => Hat,    // OH
        4 => Tom,    // T1
        5 => Tom,    // T2
        6 => Tom,    // T3
        7 => Clap,   // CL
        8 => Cymbal, // RD
        9 => Cymbal, // CY
        10 => Perc,  // S6
        11 => Perc,  // B8
        12 => Perc,  // P1
        _ => Perc,
    }
}

pub fn instrument_label(idx: usize) -> &'static str {
    match idx {
        0 => "BD",
        1 => "SD",
        2 => "HH",
        3 => "OH",
        4 => "T1",
        5 => "T2",
        6 => "T3",
        7 => "CL",
        8 => "RD",
        9 => "CY",
        10 => "S6",
        11 => "B8",
        12 => "P1",
        _ => "??",
    }
}

pub fn instrument_name(idx: usize) -> &'static str {
    match idx {
        0 => "Bass Drum",
        1 => "Snare",
        2 => "Closed Hat",
        3 => "Open Hat",
        4 => "Tom 1",
        5 => "Tom 2",
        6 => "Tom 3",
        7 => "Clap",
        8 => "Ride",
        9 => "Crash",
        10 => "Shaker",
        11 => "Perc 808",
        12 => "Perc 1",
        _ => "Unknown",
    }
}
