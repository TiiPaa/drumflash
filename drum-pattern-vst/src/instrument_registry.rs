//! Instrument registry — single source of truth for all instrument metadata.
//!
//! Adding a new instrument should only require:
//! 1. Adding a variant to `DrumVoice`
//! 2. Adding fields to `DrumFlashParams` (nih-plug constraint)
//! 3. Adding an entry to `INSTRUMENTS` below
//!
//! Everything else (multi-out, plock, UI, MIDI, parameters) is derived automatically.

use crate::synthesis::DrumVoice;

pub const SOUND_SETTINGS_FIELD_COUNT: usize = 13;

/// Functional family of a parameter, used by the Sound Panel to group sliders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamFamily {
    /// Oscillator / source parameters: pitch, algo, timbre, special tone controls.
    Osc,
    /// Amplitude envelope parameters: attack, decay, release, curves, hold.
    Env,
    /// Filter parameters: cutoff, filter envelope amount/decay.
    Filter,
    /// Saturation / distortion parameters: type, amount, mix, output gain, pre/post filter.
    Saturation,
    /// Output / routing parameters: volume, mix, stereo, analog drift.
    Output,
}

impl ParamFamily {
    pub fn label(&self) -> &'static str {
        match self {
            ParamFamily::Osc => "OSC",
            ParamFamily::Env => "ENV",
            ParamFamily::Filter => "FILTER",
            ParamFamily::Saturation => "SAT",
            ParamFamily::Output => "OUTPUT",
        }
    }
}

/// Standard sound-setting field index (matches the persistent f32 array order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StandardField {
    Freq = 0,
    Decay = 1,
    Volume = 2,
    FilterFreq = 3,
    Attack = 4,
    Release = 5,
    DecayCurve = 6,
    ReleaseCurve = 7,
    Hold = 8,
    FilterEnvAmount = 9,
    FilterEnvDecay = 10,
    Analog = 11,
    Stereo = 12,
}

impl StandardField {
    /// Returns the plock field index used in `PlockState`.
    /// This mapping aligns with how `PlockState::get_settings` and `set_settings`
    /// store values internally (see `plock.rs`).
    pub const fn plock_field_index(self) -> usize {
        match self {
            StandardField::Freq => 0,
            StandardField::Decay => 1,
            StandardField::Volume => 2,
            StandardField::FilterFreq => 3,
            StandardField::Release => 4,
            StandardField::DecayCurve => 5,
            StandardField::ReleaseCurve => 6,
            StandardField::Hold => 7,
            StandardField::FilterEnvAmount => 8,
            StandardField::FilterEnvDecay => 9,
            StandardField::Analog => 10,
            StandardField::Stereo => 11,
            StandardField::Attack => 18,
        }
    }
}

/// Widget kind for a standard parameter.
#[derive(Clone, Copy, Debug)]
pub enum ParamWidget {
    Slider {
        min: f32,
        max: f32,
        logarithmic: bool,
        suffix: Option<&'static str>,
    },
    Checkbox,
}

/// Metadata for a standard (per-instrument) parameter exposed in the Sound Panel.
#[allow(dead_code)]
pub struct StandardParamDef {
    pub field: StandardField,
    pub label: &'static str,
    pub family: ParamFamily,
    pub widget: ParamWidget,
}

/// Metadata for a special (per-instrument) parameter exposed in the Sound Panel.
#[allow(dead_code)]
pub struct SpecialParamDef {
    pub name: &'static str,
    pub label: &'static str,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    pub special_index: usize,
    pub family: ParamFamily,
    pub continuous: bool,
}

/// Helper for continuous special parameters (morphable).
#[allow(dead_code)]
const fn sp(
    name: &'static str,
    label: &'static str,
    default: f32,
    min: f32,
    max: f32,
    special_index: usize,
    family: ParamFamily,
) -> SpecialParamDef {
    SpecialParamDef {
        name,
        label,
        default,
        min,
        max,
        special_index,
        family,
        continuous: true,
    }
}

/// Helper for discrete special parameters (not morphable).
#[allow(dead_code)]
const fn sp_discrete(
    name: &'static str,
    label: &'static str,
    default: f32,
    min: f32,
    max: f32,
    special_index: usize,
    family: ParamFamily,
) -> SpecialParamDef {
    SpecialParamDef {
        name,
        label,
        default,
        min,
        max,
        special_index,
        family,
        continuous: false,
    }
}

/// Metadata for an instrument in the registry.
#[allow(dead_code)]
pub struct InstrumentDef {
    pub index: usize,
    pub name: &'static str,
    pub label: &'static str,
    pub full_name: &'static str,
    pub midi_note: u8,
    pub algo_count: usize,
    pub standard_params: &'static [StandardParamDef],
    pub special_params: &'static [SpecialParamDef],
    pub sound_settings_default: [f32; SOUND_SETTINGS_FIELD_COUNT],
    pub filter_type_label: &'static str,
    /// Ratio applied to the frequency value before displaying as note.
    /// e.g. 0.3 for Kick because the sustain freq is 0.3x the setting.
    pub freq_display_ratio: f32,
}

const fn s(
    field: StandardField,
    label: &'static str,
    family: ParamFamily,
    min: f32,
    max: f32,
    logarithmic: bool,
    suffix: Option<&'static str>,
) -> StandardParamDef {
    StandardParamDef {
        field,
        label,
        family,
        widget: ParamWidget::Slider {
            min,
            max,
            logarithmic,
            suffix,
        },
    }
}

const fn cb(field: StandardField, label: &'static str, family: ParamFamily) -> StandardParamDef {
    StandardParamDef {
        field,
        label,
        family,
        widget: ParamWidget::Checkbox,
    }
}

/// Generic standard params for instruments that support everything.
const FULL_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Frequency",
        ParamFamily::Osc,
        20.0,
        12000.0,
        true,
        None,
    ),
    s(
        StandardField::Attack,
        "Attack",
        ParamFamily::Env,
        0.0,
        0.2,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.001,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Hold,
        "Hold",
        ParamFamily::Env,
        0.0,
        2.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Release,
        "Release",
        ParamFamily::Env,
        0.0,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::ReleaseCurve,
        "Release Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Volume,
        "Volume",
        ParamFamily::Output,
        0.0,
        2.0,
        false,
        None,
    ),
    s(
        StandardField::FilterFreq,
        "Filter",
        ParamFamily::Filter,
        20.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Output,
        0.0,
        1.0,
        false,
        None,
    ),
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
];

/// Kick-like: no hold, no filter env, no stereo.
const KICK_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Frequency",
        ParamFamily::Osc,
        20.0,
        12000.0,
        true,
        None,
    ),
    s(
        StandardField::Attack,
        "Attack",
        ParamFamily::Env,
        0.0,
        0.2,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.001,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Release,
        "Release",
        ParamFamily::Env,
        0.0,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::ReleaseCurve,
        "Release Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Volume,
        "Volume",
        ParamFamily::Output,
        0.0,
        2.0,
        false,
        None,
    ),
    s(
        StandardField::FilterFreq,
        "Filter",
        ParamFamily::Filter,
        20.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Output,
        0.0,
        1.0,
        false,
        None,
    ),
];

/// Open-hat / ride / cymbal / clap: no hold, no filter env.
const NO_HOLD_NO_FILTENV_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Frequency",
        ParamFamily::Osc,
        20.0,
        12000.0,
        true,
        None,
    ),
    s(
        StandardField::Attack,
        "Attack",
        ParamFamily::Env,
        0.0,
        0.2,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.001,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Release,
        "Release",
        ParamFamily::Env,
        0.0,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::ReleaseCurve,
        "Release Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Volume,
        "Volume",
        ParamFamily::Output,
        0.0,
        2.0,
        false,
        None,
    ),
    s(
        StandardField::FilterFreq,
        "Filter",
        ParamFamily::Filter,
        20.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Output,
        0.0,
        1.0,
        false,
        None,
    ),
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
];

/// Cymbal-specific: no frequency (noise-based), no hold, no filter env.
const NO_FREQ_STD: &[StandardParamDef] = &[
    s(
        StandardField::Attack,
        "Attack",
        ParamFamily::Env,
        0.0,
        0.2,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.001,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Release,
        "Release",
        ParamFamily::Env,
        0.0,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::ReleaseCurve,
        "Release Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Volume,
        "Volume",
        ParamFamily::Output,
        0.0,
        2.0,
        false,
        None,
    ),
    s(
        StandardField::FilterFreq,
        "Filter",
        ParamFamily::Filter,
        20.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Output,
        0.0,
        1.0,
        false,
        None,
    ),
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
];

/// Tom-like: no hold, filter env, no stereo.
const TOM_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Frequency",
        ParamFamily::Osc,
        20.0,
        12000.0,
        true,
        None,
    ),
    s(
        StandardField::Attack,
        "Attack",
        ParamFamily::Env,
        0.0,
        0.2,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.001,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Release,
        "Release",
        ParamFamily::Env,
        0.0,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::ReleaseCurve,
        "Release Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Volume,
        "Volume",
        ParamFamily::Output,
        0.0,
        2.0,
        false,
        None,
    ),
    s(
        StandardField::FilterFreq,
        "Filter",
        ParamFamily::Filter,
        20.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::FilterEnvAmount,
        "Filter Env",
        ParamFamily::Filter,
        0.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::FilterEnvDecay,
        "Filter Decay",
        ParamFamily::Filter,
        0.001,
        2.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Output,
        0.0,
        1.0,
        false,
        None,
    ),
];

/// Snare606: hold, no filter env, stereo-capable.
const SNARE606_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Frequency",
        ParamFamily::Osc,
        20.0,
        12000.0,
        true,
        None,
    ),
    s(
        StandardField::Attack,
        "Attack",
        ParamFamily::Env,
        0.0,
        0.2,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.001,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Hold,
        "Hold",
        ParamFamily::Env,
        0.0,
        2.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Release,
        "Release",
        ParamFamily::Env,
        0.0,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::ReleaseCurve,
        "Release Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Volume,
        "Volume",
        ParamFamily::Output,
        0.0,
        2.0,
        false,
        None,
    ),
    s(
        StandardField::FilterFreq,
        "Filter",
        ParamFamily::Filter,
        20.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Output,
        0.0,
        1.0,
        false,
        None,
    ),
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
];

/// B8: no hold, no filter env, no stereo.
const MINIMAL_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Frequency",
        ParamFamily::Osc,
        20.0,
        12000.0,
        true,
        None,
    ),
    s(
        StandardField::Attack,
        "Attack",
        ParamFamily::Env,
        0.0,
        0.2,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.001,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Release,
        "Release",
        ParamFamily::Env,
        0.0,
        5.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::ReleaseCurve,
        "Release Curve",
        ParamFamily::Env,
        0.1,
        20.0,
        false,
        None,
    ),
    s(
        StandardField::Volume,
        "Volume",
        ParamFamily::Output,
        0.0,
        2.0,
        false,
        None,
    ),
    s(
        StandardField::FilterFreq,
        "Filter",
        ParamFamily::Filter,
        20.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Output,
        0.0,
        1.0,
        false,
        None,
    ),
];

pub const INSTRUMENTS: [InstrumentDef; DrumVoice::COUNT] = [
    InstrumentDef {
        index: 0,
        name: "Kick",
        label: "BD",
        full_name: "Kick Drum",
        midi_note: 36,
        algo_count: 3,
        standard_params: KICK_STD,
        special_params: &[
            sp(
        "kick_click",
        "Click Level",
        0.5,
        0.0,
        1.0,
        0,
        ParamFamily::Osc,
    ),
            sp_discrete(
        "kick_click_type",
        "Click Type",
        1.0,
        0.0,
        2.0,
        6,
        ParamFamily::Osc,
    ),
            sp_discrete(
        "kick_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "kick_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "kick_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        3,
        ParamFamily::Saturation,
    ),
            sp(
        "kick_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        4,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "kick_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
        ],
        // Kick-like: no hold, no filter env, no stereo
        sound_settings_default: [
            60.0, 0.5, 0.8, 30.0, 0.0015, 0.5, 5.0, 3.0, 0.0, 1.0, 0.05, 0.3, 0.0,
        ],
        freq_display_ratio: 0.3,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 1,
        name: "Snare",
        label: "SD",
        full_name: "Snare Drum",
        midi_note: 38,
        algo_count: 3,
        standard_params: FULL_STD,
        special_params: &[
            sp(
        "snare_snap",
        "Snap",
        0.5,
        0.0,
        1.0,
        0,
        ParamFamily::Osc,
    ),
            sp_discrete(
        "snare_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "snare_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "snare_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        3,
        ParamFamily::Saturation,
    ),
            sp(
        "snare_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        4,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "snare_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
        ],
        // Full: hold, filter env, stereo
        sound_settings_default: [
            200.0, 0.47, 0.6, 200.0, 0.0003, 0.2, 5.0, 3.0, 0.0, 1.0, 0.03, 0.3, 1.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 2,
        name: "HiHat",
        label: "HH",
        full_name: "Hi-Hat",
        midi_note: 42,
        algo_count: 2,
        standard_params: FULL_STD,
        special_params: &[
            sp_discrete(
        "hihat_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        0,
        ParamFamily::Saturation,
    ),
            sp(
        "hihat_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "hihat_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "hihat_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        3,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "hihat_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        4,
        ParamFamily::Saturation,
    ),
        ],
        // Full: hold, filter env, stereo
        sound_settings_default: [
            8000.0, 0.36, 0.3, 5000.0, 0.0003, 0.0, 8.0, 3.0, 0.0, 1.0, 0.04, 1.0, 1.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 3,
        name: "OpenHiHat",
        label: "OH",
        full_name: "Open Hi-Hat",
        midi_note: 46,
        algo_count: 2,
        standard_params: NO_HOLD_NO_FILTENV_STD,
        special_params: &[
            sp_discrete(
        "openhihat_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        0,
        ParamFamily::Saturation,
    ),
            sp(
        "openhihat_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "openhihat_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "openhihat_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        3,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "openhihat_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        4,
        ParamFamily::Saturation,
    ),
        ],
        // No hold, no filter env, stereo-capable
        sound_settings_default: [
            6000.0, 0.66, 0.4, 8000.0, 0.0003, 0.4, 5.5, 3.0, 0.0, 0.0, 0.05, 1.0, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 4,
        name: "Tom1",
        label: "T1",
        full_name: "Tom 1",
        midi_note: 50,
        algo_count: 2,
        standard_params: TOM_STD,
        special_params: &[
            sp(
        "tom_stick",
        "Stick Attack",
        0.5,
        0.0,
        1.0,
        0,
        ParamFamily::Osc,
    ),
            sp_discrete(
        "tom_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        3,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        4,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "tom_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
        ],
        // No hold, filter env, no stereo
        sound_settings_default: [
            300.0, 0.3, 0.5, 500.0, 0.0015, 0.3, 4.2, 3.0, 0.0, 1.0, 0.06, 0.3, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 5,
        name: "Tom2",
        label: "T2",
        full_name: "Tom 2",
        midi_note: 47,
        algo_count: 2,
        standard_params: TOM_STD,
        special_params: &[
            sp(
        "tom_stick",
        "Stick Attack",
        0.5,
        0.0,
        1.0,
        0,
        ParamFamily::Osc,
    ),
            sp_discrete(
        "tom_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        3,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        4,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "tom_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
        ],
        // No hold, filter env, no stereo
        sound_settings_default: [
            200.0, 0.4, 0.5, 500.0, 0.0015, 0.4, 4.2, 3.0, 0.0, 1.0, 0.06, 0.3, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 6,
        name: "Tom3",
        label: "T3",
        full_name: "Tom 3",
        midi_note: 43,
        algo_count: 2,
        standard_params: TOM_STD,
        special_params: &[
            sp(
        "tom_stick",
        "Stick Attack",
        0.5,
        0.0,
        1.0,
        0,
        ParamFamily::Osc,
    ),
            sp_discrete(
        "tom_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        3,
        ParamFamily::Saturation,
    ),
            sp(
        "tom_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        4,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "tom_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
        ],
        // No hold, filter env, no stereo
        sound_settings_default: [
            120.0, 0.5, 0.5, 500.0, 0.0015, 0.5, 4.2, 3.0, 0.0, 1.0, 0.06, 0.3, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 7,
        name: "Clap",
        label: "CL",
        full_name: "Clap",
        midi_note: 39,
        algo_count: 2,
        standard_params: NO_FREQ_STD,
        special_params: &[
            sp(
        "clap_echo",
        "Echo",
        0.5,
        0.0,
        3.0,
        0,
        ParamFamily::Env,
    ),
            sp_discrete(
        "clap_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "clap_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "clap_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        3,
        ParamFamily::Saturation,
    ),
            sp(
        "clap_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        4,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "clap_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
        ],
        // No hold, no filter env, stereo
        sound_settings_default: [
            1200.0, 0.03, 0.7, 1000.0, 0.0015, 0.12, 6.0, 3.0, 0.0, 0.0, 0.05, 1.0, 1.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 8,
        name: "Ride",
        label: "RD",
        full_name: "Ride",
        midi_note: 51,
        algo_count: 2,
        standard_params: NO_HOLD_NO_FILTENV_STD,
        special_params: &[
            sp_discrete(
        "ride_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        0,
        ParamFamily::Saturation,
    ),
            sp(
        "ride_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        1,
        ParamFamily::Saturation,
    ),
            sp(
        "ride_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        2,
        ParamFamily::Saturation,
    ),
            sp(
        "ride_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        3,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "ride_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        4,
        ParamFamily::Saturation,
    ),
        ],
        // No hold, no filter env, stereo (default mono for stability)
        sound_settings_default: [
            8000.0, 1.2, 0.35, 10000.0, 0.002, 1.5, 3.5, 3.0, 0.0, 0.0, 0.05, 1.0, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 9,
        name: "Cymbal",
        label: "CY",
        full_name: "Cymbal",
        midi_note: 49,
        algo_count: 1,
        standard_params: NO_FREQ_STD,
        special_params: &[
            sp_discrete(
        "cymbal_noise_type",
        "Noise Type",
        0.0,
        0.0,
        3.0,
        1,
        ParamFamily::Osc,
    ),
            sp(
        "cymbal_shimmer_freq",
        "Shimmer Freq",
        15.0,
        1.0,
        50.0,
        0,
        ParamFamily::Osc,
    ),
            sp(
        "cymbal_shimmer_amount",
        "Shimmer Amount",
        0.15,
        0.0,
        1.0,
        2,
        ParamFamily::Osc,
    ),
            sp_discrete(
        "cymbal_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        3,
        ParamFamily::Saturation,
    ),
            sp(
        "cymbal_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        4,
        ParamFamily::Saturation,
    ),
            sp(
        "cymbal_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
            sp(
        "cymbal_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        6,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "cymbal_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        7,
        ParamFamily::Saturation,
    ),
        ],
        // No hold, no filter env, stereo (default mono for stability)
        sound_settings_default: [
            6000.0, 2.0, 0.4, 8000.0, 0.002, 2.5, 2.8, 3.0, 0.0, 0.0, 0.05, 0.3, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "HP",
    },
    InstrumentDef {
        index: 10,
        name: "Snare606",
        label: "S6",
        full_name: "Snare 606",
        midi_note: 40,
        algo_count: 2,
        standard_params: SNARE606_STD,
        special_params: &[
            sp(
        "snare606_resonance",
        "Resonance",
        4.5,
        0.5,
        12.0,
        0,
        ParamFamily::Filter,
    ),
            sp(
        "snare606_tone",
        "Tone",
        0.55,
        0.0,
        1.0,
        1,
        ParamFamily::Osc,
    ),
            sp(
        "snare606_snap",
        "Snap",
        0.3,
        0.0,
        1.0,
        2,
        ParamFamily::Osc,
    ),
            sp_discrete(
        "snare606_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        3,
        ParamFamily::Saturation,
    ),
            sp(
        "snare606_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        4,
        ParamFamily::Saturation,
    ),
            sp(
        "snare606_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
            sp(
        "snare606_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        6,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "snare606_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        7,
        ParamFamily::Saturation,
    ),
        ],
        // Snare606: hold, no filter env, stereo-capable
        sound_settings_default: [
            220.0, 0.08, 0.7, 3000.0, 0.0003, 0.15, 5.0, 3.0, 0.0, 0.0, 0.05, 1.0, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 11,
        name: "BassDrum808",
        label: "B8",
        full_name: "808 Kick",
        midi_note: 35,
        algo_count: 2,
        standard_params: MINIMAL_STD,
        special_params: &[
            sp(
        "bassdrum808_accent",
        "Accent",
        0.0,
        0.0,
        2.0,
        0,
        ParamFamily::Osc,
    ),
            sp(
        "bassdrum808_snap",
        "Snap",
        0.0,
        0.0,
        2.0,
        1,
        ParamFamily::Osc,
    ),
            sp(
        "bassdrum808_pitch_drop",
        "Pitch Drop",
        0.0,
        0.0,
        2.0,
        2,
        ParamFamily::Osc,
    ),
            sp(
        "bassdrum808_click_tone",
        "Click Tone",
        4000.0,
        100.0,
        8000.0,
        3,
        ParamFamily::Filter,
    ),
            sp_discrete(
        "bassdrum808_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        4,
        ParamFamily::Saturation,
    ),
            sp(
        "bassdrum808_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
            sp(
        "bassdrum808_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        6,
        ParamFamily::Saturation,
    ),
            sp(
        "bassdrum808_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        7,
        ParamFamily::Saturation,
    ),
        ],
        // Minimal: no hold, no filter env, no stereo
        sound_settings_default: [
            50.0, 0.4, 0.9, 3000.0, 0.0015, 0.0, 3.0, 3.0, 0.0, 0.0, 0.05, 0.3, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 12,
        name: "Perc1",
        label: "P1",
        full_name: "Perc1",
        midi_note: 37,
        algo_count: 2,
        standard_params: FULL_STD,
        special_params: &[
            sp(
        "perc1_sweep",
        "Sweep",
        0.5,
        -1.0,
        1.0,
        0,
        ParamFamily::Osc,
    ),
            sp(
        "perc1_speed",
        "Speed",
        80.0,
        5.0,
        300.0,
        1,
        ParamFamily::Osc,
    ),
            sp(
        "perc1_bite",
        "Bite",
        0.0,
        0.0,
        1.0,
        2,
        ParamFamily::Osc,
    ),
            sp(
        "perc1_width",
        "Width",
        0.0,
        0.0,
        1.0,
        3,
        ParamFamily::Output,
    ),
            sp_discrete(
        "perc1_saturation_type",
        "Saturation Type",
        0.0,
        0.0,
        5.0,
        4,
        ParamFamily::Saturation,
    ),
            sp(
        "perc1_saturation_amount",
        "Saturation Amount",
        0.0,
        0.0,
        1.0,
        5,
        ParamFamily::Saturation,
    ),
            sp(
        "perc1_saturation_mix",
        "Saturation Mix",
        1.0,
        0.0,
        1.0,
        6,
        ParamFamily::Saturation,
    ),
            sp(
        "perc1_saturation_output_gain",
        "Saturation Output Gain",
        1.0,
        0.5,
        2.0,
        7,
        ParamFamily::Saturation,
    ),
            sp_discrete(
        "perc1_saturation_pre_filter",
        "Saturation Pre-Filter",
        0.0,
        0.0,
        1.0,
        8,
        ParamFamily::Saturation,
    ),
        ],
        // Full: filter env, stereo
        sound_settings_default: [
            2000.0, 0.15, 0.6, 6000.0, 0.0005, 0.0, 5.0, 3.0, 0.0, 0.7, 0.03, 0.3, 1.0,
        ],
        freq_display_ratio: 1.0,
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

/// Highest algo index across all instruments. Used as the shared range of the
/// per-slot algo params (any kind can live on any slot); kept >= 1 because an
/// IntRange with min == max crashes nih-plug normalization (bug [42]).
pub fn max_algo_index() -> i32 {
    INSTRUMENTS
        .iter()
        .map(|i| i.algo_count)
        .max()
        .unwrap_or(2)
        .saturating_sub(1)
        .max(1) as i32
}

pub fn special_params(voice_idx: usize) -> &'static [SpecialParamDef] {
    INSTRUMENTS[voice_idx].special_params
}

#[allow(dead_code)]
pub fn sound_settings_default(voice_idx: usize) -> &'static [f32; SOUND_SETTINGS_FIELD_COUNT] {
    &INSTRUMENTS[voice_idx].sound_settings_default
}

#[allow(dead_code)]
pub fn filter_type_label(voice_idx: usize) -> &'static str {
    INSTRUMENTS[voice_idx].filter_type_label
}

#[allow(dead_code)]
pub struct MorphableField {
    pub field_index: usize,
    pub label: &'static str,
    pub min: f32,
    pub max: f32,
}

const SPECIAL_FIELD_START: usize = 14;

/// Returns all plock fields that support continuous morphing for a given instrument.
/// Includes all standard (slider) fields and continuous special parameters.
#[allow(dead_code)]
pub fn morphable_fields(voice_idx: usize) -> Vec<MorphableField> {
    if voice_idx >= INSTRUMENTS.len() {
        return Vec::new();
    }
    let inst = &INSTRUMENTS[voice_idx];
    let mut fields = Vec::new();

    for def in inst.standard_params {
        if let ParamWidget::Slider { min, max, .. } = def.widget {
            fields.push(MorphableField {
                field_index: def.field.plock_field_index(),
                label: def.label,
                min,
                max,
            });
        }
    }

    for def in inst.special_params {
        if def.continuous {
            fields.push(MorphableField {
                field_index: SPECIAL_FIELD_START + def.special_index,
                label: def.label,
                min: def.min,
                max: def.max,
            });
        }
    }

    fields
}

/// Map an incoming MIDI note number to a voice index.
/// Returns `Some(index)` if the note matches one of the instrument's default
/// MIDI notes, `None` otherwise.
pub fn voice_idx_from_midi_note(note: u8) -> Option<usize> {
    INSTRUMENTS.iter().position(|inst| inst.midi_note == note)
}
