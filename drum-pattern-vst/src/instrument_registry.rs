//! Instrument registry — single source of truth for all instrument metadata.
//!
//! Adding a new instrument should only require:
//! 1. Adding a variant to `DrumVoice`
//! 2. Adding fields to `DrumFlashParams` (nih-plug constraint)
//! 3. Adding an entry to `INSTRUMENTS` below
//!
//! Everything else (multi-out, plock, UI, MIDI, parameters) is derived automatically.

use crate::synthesis::DrumVoice;

#[allow(dead_code)]
pub struct SpecialParamDef {
    pub name: &'static str,
    pub label: &'static str,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    pub special_index: usize,
}

pub struct InstrumentCapabilities {
    pub freq: bool,
    pub hold: bool,
    pub filter_env: bool,
    pub analog: bool,
    pub stereo: bool,
}

#[allow(dead_code)]
pub struct InstrumentDef {
    pub index: usize,
    pub name: &'static str,
    pub label: &'static str,
    pub full_name: &'static str,
    pub midi_note: u8,
    pub algo_count: usize,
    pub special_params: &'static [SpecialParamDef],
    pub capabilities: InstrumentCapabilities,
    pub sound_settings_default: [f32; 12],
    pub filter_type_label: &'static str,
}

pub const INSTRUMENTS: [InstrumentDef; DrumVoice::COUNT] = [
    InstrumentDef {
        index: 0,
        name: "Kick",
        label: "BD",
        full_name: "Kick Drum",
        midi_note: 36,
        algo_count: 3,
        special_params: &[
            SpecialParamDef { name: "kick_click", label: "Click Level", default: 0.5, min: 0.0, max: 1.0, special_index: 0 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: false, analog: true, stereo: false },
        sound_settings_default: [60.0, 0.5, 0.8, 30.0, 0.5, 5.0, 3.0, 0.0, 1.0, 0.05, 1.0, 0.0],
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 1,
        name: "Snare",
        label: "SD",
        full_name: "Snare Drum",
        midi_note: 38,
        algo_count: 3,
        special_params: &[
            SpecialParamDef { name: "snare_snap", label: "Snap", default: 0.5, min: 0.0, max: 1.0, special_index: 0 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: true, filter_env: true, analog: true, stereo: true },
        sound_settings_default: [200.0, 0.47, 0.6, 200.0, 0.2, 5.0, 3.0, 0.0, 1.0, 0.03, 1.0, 1.0],
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 2,
        name: "HiHat",
        label: "HH",
        full_name: "Hi-Hat",
        midi_note: 42,
        algo_count: 2,
        special_params: &[],
        capabilities: InstrumentCapabilities { freq: true, hold: true, filter_env: true, analog: true, stereo: true },
        sound_settings_default: [8000.0, 0.36, 0.3, 5000.0, 0.0, 8.0, 3.0, 0.0, 1.0, 0.04, 1.0, 1.0],
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 3,
        name: "OpenHiHat",
        label: "OH",
        full_name: "Open Hi-Hat",
        midi_note: 46,
        algo_count: 2,
        special_params: &[],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: false, analog: true, stereo: false },
        sound_settings_default: [6000.0, 0.66, 0.4, 8000.0, 0.4, 5.5, 3.0, 0.0, 0.0, 0.05, 1.0, 0.0],
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 4,
        name: "Tom1",
        label: "T1",
        full_name: "Tom 1",
        midi_note: 50,
        algo_count: 2,
        special_params: &[
            SpecialParamDef { name: "tom_stick", label: "Stick Attack", default: 0.5, min: 0.0, max: 1.0, special_index: 0 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: true, analog: true, stereo: false },
        sound_settings_default: [300.0, 0.3, 0.5, 500.0, 0.3, 4.2, 3.0, 0.0, 1.0, 0.06, 1.0, 0.0],
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 5,
        name: "Tom2",
        label: "T2",
        full_name: "Tom 2",
        midi_note: 47,
        algo_count: 2,
        special_params: &[
            SpecialParamDef { name: "tom_stick", label: "Stick Attack", default: 0.5, min: 0.0, max: 1.0, special_index: 0 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: true, analog: true, stereo: false },
        sound_settings_default: [200.0, 0.4, 0.5, 500.0, 0.4, 4.2, 3.0, 0.0, 1.0, 0.06, 1.0, 0.0],
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 6,
        name: "Tom3",
        label: "T3",
        full_name: "Tom 3",
        midi_note: 43,
        algo_count: 2,
        special_params: &[
            SpecialParamDef { name: "tom_stick", label: "Stick Attack", default: 0.5, min: 0.0, max: 1.0, special_index: 0 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: true, analog: true, stereo: false },
        sound_settings_default: [120.0, 0.5, 0.5, 500.0, 0.5, 4.2, 3.0, 0.0, 1.0, 0.06, 1.0, 0.0],
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 7,
        name: "Clap",
        label: "CL",
        full_name: "Clap",
        midi_note: 39,
        algo_count: 2,
        special_params: &[
            SpecialParamDef { name: "clap_echo", label: "Echo", default: 0.5, min: 0.0, max: 3.0, special_index: 0 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: false, analog: true, stereo: true },
        sound_settings_default: [1200.0, 0.03, 0.7, 1000.0, 0.12, 6.0, 3.0, 0.0, 0.0, 0.05, 1.0, 1.0],
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 8,
        name: "Ride",
        label: "RD",
        full_name: "Ride",
        midi_note: 51,
        algo_count: 2,
        special_params: &[],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: false, analog: true, stereo: true },
        sound_settings_default: [8000.0, 1.2, 0.35, 10000.0, 1.5, 3.5, 3.0, 0.0, 0.0, 0.05, 1.0, 1.0],
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 9,
        name: "Cymbal",
        label: "CY",
        full_name: "Cymbal",
        midi_note: 49,
        algo_count: 2,
        special_params: &[],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: false, analog: true, stereo: true },
        sound_settings_default: [6000.0, 2.0, 0.4, 8000.0, 2.5, 2.8, 3.0, 0.0, 0.0, 0.05, 1.0, 1.0],
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 10,
        name: "Snare606",
        label: "S6",
        full_name: "Snare 606",
        midi_note: 40,
        algo_count: 2,
        special_params: &[
            SpecialParamDef { name: "snare606_resonance", label: "Resonance", default: 4.5, min: 0.5, max: 12.0, special_index: 0 },
            SpecialParamDef { name: "snare606_tone", label: "Tone", default: 0.55, min: 0.0, max: 1.0, special_index: 1 },
            SpecialParamDef { name: "snare606_snap", label: "Snap", default: 0.3, min: 0.0, max: 1.0, special_index: 2 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: false, analog: true, stereo: false },
        sound_settings_default: [220.0, 0.08, 0.7, 3000.0, 0.15, 5.0, 3.0, 0.0, 0.0, 0.05, 1.0, 0.0],
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 11,
        name: "BassDrum808",
        label: "B8",
        full_name: "808 Kick",
        midi_note: 35,
        algo_count: 2,
        special_params: &[
            SpecialParamDef { name: "bassdrum808_accent", label: "Accent", default: 0.0, min: 0.0, max: 2.0, special_index: 0 },
            SpecialParamDef { name: "bassdrum808_snap", label: "Snap", default: 0.0, min: 0.0, max: 2.0, special_index: 1 },
            SpecialParamDef { name: "bassdrum808_pitch_drop", label: "Pitch Drop", default: 0.0, min: 0.0, max: 2.0, special_index: 2 },
            SpecialParamDef { name: "bassdrum808_click_tone", label: "Click Tone", default: 4000.0, min: 100.0, max: 8000.0, special_index: 3 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: false, analog: true, stereo: false },
        sound_settings_default: [50.0, 0.4, 0.9, 3000.0, 0.0, 3.0, 3.0, 0.0, 0.0, 0.05, 1.0, 0.0],
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 12,
        name: "Zap",
        label: "ZP",
        full_name: "Zap",
        midi_note: 37,
        algo_count: 2,
        special_params: &[
            SpecialParamDef { name: "zap_sweep", label: "Sweep", default: 0.0, min: -1.0, max: 1.0, special_index: 0 },
            SpecialParamDef { name: "zap_speed", label: "Speed", default: 5.0, min: 0.5, max: 50.0, special_index: 1 },
            SpecialParamDef { name: "zap_bite", label: "Bite", default: 0.0, min: 0.0, max: 1.0, special_index: 2 },
            SpecialParamDef { name: "zap_width", label: "Width", default: 0.0, min: 0.0, max: 1.0, special_index: 3 },
        ],
        capabilities: InstrumentCapabilities { freq: true, hold: false, filter_env: true, analog: true, stereo: true },
        sound_settings_default: [2000.0, 0.04, 0.6, 6000.0, 0.0, 5.0, 3.0, 0.0, 0.7, 0.03, 0.3, 1.0],
        filter_type_label: "LP",
    },
];

#[allow(dead_code)]
pub fn label(voice_idx: usize) -> &'static str {
    INSTRUMENTS[voice_idx].label
}

#[allow(dead_code)]
pub fn full_name(voice_idx: usize) -> &'static str {
    INSTRUMENTS[voice_idx].full_name
}

#[allow(dead_code)]
pub fn midi_note(voice_idx: usize) -> u8 {
    INSTRUMENTS[voice_idx].midi_note
}

#[allow(dead_code)]
pub fn algo_count(voice_idx: usize) -> usize {
    INSTRUMENTS[voice_idx].algo_count
}

pub fn special_params(voice_idx: usize) -> &'static [SpecialParamDef] {
    INSTRUMENTS[voice_idx].special_params
}

pub fn capabilities(voice_idx: usize) -> &'static InstrumentCapabilities {
    &INSTRUMENTS[voice_idx].capabilities
}

#[allow(dead_code)]
pub fn sound_settings_default(voice_idx: usize) -> &'static [f32; 12] {
    &INSTRUMENTS[voice_idx].sound_settings_default
}

pub fn filter_type_label(voice_idx: usize) -> &'static str {
    INSTRUMENTS[voice_idx].filter_type_label
}
