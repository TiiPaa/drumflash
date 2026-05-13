//! Special parameter definitions per instrument.
//!
//! Each instrument declares:
//! - Its supported synthesis algorithms (algos)
//! - Its special parameters (indexed 0..7 within VoiceSettings.special)
//!
//! This module is UI-facing: the editor queries these definitions to build
//! dynamic controls. The audio thread only sees `VoiceSettings.special[index]`.

/// Description of a synthesis algorithm for UI display.
#[derive(Clone, Copy, Debug)]
pub struct AlgoDef {
    pub index: u8,
    pub name: &'static str,
}

/// Description of a special parameter for UI display and defaults.
#[derive(Clone, Copy, Debug)]
pub struct SpecialParamDef {
    pub index: usize,
    pub name: &'static str,
    pub default: f32,
    pub min: f32,
    pub max: f32,
}

// ── Kick ────────────────────────────────────────────────────────────────────

pub const KICK_ALGOS: &[AlgoDef] = &[
    AlgoDef { index: 0, name: "Sine" },
    AlgoDef { index: 1, name: "Square" },
    AlgoDef { index: 2, name: "FM" },
];

pub const KICK_SPECIALS: &[SpecialParamDef] = &[
    SpecialParamDef {
        index: 0,
        name: "Click Level",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
    SpecialParamDef {
        index: 1,
        name: "Click Decay",
        default: 0.01,
        min: 0.001,
        max: 0.05,
    },
    SpecialParamDef {
        index: 2,
        name: "Punch",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
];

// ── Snare ───────────────────────────────────────────────────────────────────

pub const SNARE_ALGOS: &[AlgoDef] = &[
    AlgoDef { index: 0, name: "Synth" },
    AlgoDef { index: 1, name: "Noise" },
    AlgoDef { index: 2, name: "Layered" },
];

pub const SNARE_SPECIALS: &[SpecialParamDef] = &[
    SpecialParamDef {
        index: 0,
        name: "Snap",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
    SpecialParamDef {
        index: 1,
        name: "Tone",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
];

// ── HiHat (closed & open share the same algo/special set) ───────────────────

pub const HIHAT_ALGOS: &[AlgoDef] = &[
    AlgoDef { index: 0, name: "Standard" },
    AlgoDef { index: 1, name: "Bright" },
];

pub const HIHAT_SPECIALS: &[SpecialParamDef] = &[
    SpecialParamDef {
        index: 0,
        name: "Splash",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
];

// ── Tom ─────────────────────────────────────────────────────────────────────

pub const TOM_ALGOS: &[AlgoDef] = &[
    AlgoDef { index: 0, name: "Standard" },
    AlgoDef { index: 1, name: "Deep" },
];

pub const TOM_SPECIALS: &[SpecialParamDef] = &[
    SpecialParamDef {
        index: 0,
        name: "Stick Attack",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
];

// ── Clap ────────────────────────────────────────────────────────────────────

pub const CLAP_ALGOS: &[AlgoDef] = &[
    AlgoDef { index: 0, name: "Standard" },
    AlgoDef { index: 1, name: "Tight" },
];

pub const CLAP_SPECIALS: &[SpecialParamDef] = &[
    SpecialParamDef {
        index: 0,
        name: "Spread",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
];

// ── Ride ────────────────────────────────────────────────────────────────────

pub const RIDE_ALGOS: &[AlgoDef] = &[
    AlgoDef { index: 0, name: "Standard" },
    AlgoDef { index: 1, name: "Bell" },
];

pub const RIDE_SPECIALS: &[SpecialParamDef] = &[
    SpecialParamDef {
        index: 0,
        name: "Bell Mix",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
];

// ── Cymbal ──────────────────────────────────────────────────────────────────

pub const CYMBAL_ALGOS: &[AlgoDef] = &[
    AlgoDef { index: 0, name: "Standard" },
    AlgoDef { index: 1, name: "Dark" },
];

pub const CYMBAL_SPECIALS: &[SpecialParamDef] = &[
    SpecialParamDef {
        index: 0,
        name: "Shimmer Rate",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
    SpecialParamDef {
        index: 1,
        name: "Shimmer Depth",
        default: 0.5,
        min: 0.0,
        max: 1.0,
    },
];

// ── Registry helpers ────────────────────────────────────────────────────────

use super::DrumVoice;

/// Returns the algo definitions for a given drum voice.
pub fn algos_for(voice: DrumVoice) -> &'static [AlgoDef] {
    match voice {
        DrumVoice::Kick => KICK_ALGOS,
        DrumVoice::Snare => SNARE_ALGOS,
        DrumVoice::HiHat => HIHAT_ALGOS,
        DrumVoice::OpenHiHat => HIHAT_ALGOS,
        DrumVoice::Tom1 => TOM_ALGOS,
        DrumVoice::Tom2 => TOM_ALGOS,
        DrumVoice::Tom3 => TOM_ALGOS,
        DrumVoice::Clap => CLAP_ALGOS,
        DrumVoice::Ride => RIDE_ALGOS,
        DrumVoice::Cymbal => CYMBAL_ALGOS,
    }
}

/// Returns the special parameter definitions for a given drum voice.
pub fn specials_for(voice: DrumVoice) -> &'static [SpecialParamDef] {
    match voice {
        DrumVoice::Kick => KICK_SPECIALS,
        DrumVoice::Snare => SNARE_SPECIALS,
        DrumVoice::HiHat => HIHAT_SPECIALS,
        DrumVoice::OpenHiHat => HIHAT_SPECIALS,
        DrumVoice::Tom1 => TOM_SPECIALS,
        DrumVoice::Tom2 => TOM_SPECIALS,
        DrumVoice::Tom3 => TOM_SPECIALS,
        DrumVoice::Clap => CLAP_SPECIALS,
        DrumVoice::Ride => RIDE_SPECIALS,
        DrumVoice::Cymbal => CYMBAL_SPECIALS,
    }
}
