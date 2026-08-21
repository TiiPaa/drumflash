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
    /// Analog / character parameters: the global analog/digital switch.
    Analog,
    /// Filter parameters: cutoff, filter envelope amount/decay.
    Filter,
    /// Modulation parameters: target, LFO, flanger and modulation depth/mix.
    Modulation,
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
            ParamFamily::Analog => "ANALOG",
            ParamFamily::Filter => "FILTER",
            ParamFamily::Modulation => "MOD",
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
    /// Display unit, e.g. `Some(" Hz")` ([182]). `None` for dimensionless
    /// amounts (depth, wet, mix…), exactly like a standard param's `suffix`.
    pub unit: Option<&'static str>,
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
        unit: None,
    }
}

/// Helper for a continuous special parameter that carries a display unit ([182]).
#[allow(dead_code)]
const fn sp_unit(
    name: &'static str,
    label: &'static str,
    default: f32,
    min: f32,
    max: f32,
    special_index: usize,
    family: ParamFamily,
    unit: &'static str,
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
        unit: Some(unit),
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
        unit: None,
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
        -1.0,
        1.0,
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
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        ParamFamily::Analog,
        0.0,
        1.0,
        false,
        None,
    ),
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
];

/// Buzz-specific: amp is A-H-D (no release), plus an A-H-D filter envelope
/// (Filter Env amount + Filter Decay standard, Filter Attack/Hold as specials).
const BUZZ_STD: &[StandardParamDef] = &[
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
        -1.0,
        1.0,
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
        0.01,
        1.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Analog,
        0.0,
        1.0,
        false,
        None,
    ),
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
];

/// HiHat-specific: like FULL_STD but the Freq knob controls the peaking filter
/// center (the "metallic tone" of the noise) rather than an oscillator pitch.
const HIHAT_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Tone",
        ParamFamily::Osc,
        100.0,
        20000.0,
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
        -1.0,
        1.0,
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
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        "Cutoff",
        ParamFamily::Filter,
        100.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Analog,
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
        2.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        ParamFamily::Analog,
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
        -1.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        ParamFamily::Analog,
        0.0,
        1.0,
        false,
        None,
    ),
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
];

/// Open-hat specific: like NO_HOLD_NO_FILTENV_STD but the Freq knob controls the
/// peaking filter center (the "metallic tone" of the noise).
const OPENHIHAT_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Tone",
        ParamFamily::Osc,
        100.0,
        20000.0,
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
        -1.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        "Cutoff",
        ParamFamily::Filter,
        100.0,
        20000.0,
        true,
        Some(" Hz"),
    ),
    s(
        StandardField::Analog,
        "Analog",
        ParamFamily::Analog,
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
        -1.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        ParamFamily::Analog,
        0.0,
        1.0,
        false,
        None,
    ),
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
];

/// Clap: same as `NO_FREQ_STD` but the decay caps at 1.5 s ([181]) — a clap
/// never needs more, and the Cymbal keeps the long 5 s range.
const CLAP_STD: &[StandardParamDef] = &[
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
        1.5,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        ParamFamily::Analog,
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
        -1.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        ParamFamily::Analog,
        0.0,
        1.0,
        false,
        None,
    ),
];

/// 606 multisample voices (BD6smp / SD6smp): pitch in semitones relative to
/// the native sample rate, amp/filter envelope times as FRACTIONS of the
/// played sample length (no Release � the sample tail is the release).
const SMP606_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Pitch",
        ParamFamily::Osc,
        -24.0,
        24.0,
        false,
        None,
    ),
    s(
        StandardField::Attack,
        "Attack",
        ParamFamily::Env,
        0.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.01,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
    // [168] Stereo = two distinct samples L&R in pairs (1+2, 3+4, 5+6, 7+8);
    // with Analog Mode on, a random pair plays per hit. Rendered in the UI
    // directly under the Sample select.
    cb(StandardField::Stereo, "Stereo", ParamFamily::Output),
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
        0.01,
        1.0,
        false,
        None,
    ),
];

/// SDrex: body/noise/metal recipe + volume A-D and LP filter A-D envelopes.
const SDREX_STD: &[StandardParamDef] = &[
    s(
        StandardField::Freq,
        "Frequency",
        ParamFamily::Osc,
        60.0,
        600.0,
        true,
        Some(" Hz"),
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
        StandardField::Hold,
        "Hold",
        ParamFamily::Env,
        0.0,
        1.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::Decay,
        "Decay",
        ParamFamily::Env,
        0.03,
        1.5,
        false,
        Some(" s"),
    ),
    s(
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        StandardField::Analog,
        "Analog",
        ParamFamily::Analog,
        0.0,
        1.0,
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
        1.5,
        false,
        Some(" s"),
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
        -1.0,
        1.0,
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
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        ParamFamily::Analog,
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
        2.0,
        false,
        Some(" s"),
    ),
    s(
        StandardField::DecayCurve,
        "Decay Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
        false,
        None,
    ),
    s(
        StandardField::ReleaseCurve,
        "Attack Curve",
        ParamFamily::Env,
        -1.0,
        1.0,
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
        ParamFamily::Analog,
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
            60.0, 0.5, 1.0, 30.0, 0.0015, 0.5, 5.0, 3.0, 0.0, 1.0, 0.05, 0.5, 0.0,
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
            sp("snare_snap", "Snap", 0.5, 0.0, 1.0, 0, ParamFamily::Osc),
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
            200.0, 0.47, 0.6, 200.0, 0.0003, 0.2, 5.0, 3.0, 0.0, 1.0, 0.03, 0.5, 1.0,
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
        algo_count: 1,
        standard_params: HIHAT_STD,
        special_params: &[
            sp_discrete(
                "hihat_noise_type",
                "Noise Type",
                0.0,
                0.0,
                3.0,
                5,
                ParamFamily::Osc,
            ),
            sp(
                "hihat_resonance",
                "Resonance",
                2.0,
                0.1,
                10.0,
                6,
                ParamFamily::Osc,
            ),
            sp(
                "hihat_shimmer",
                "Shimmer",
                0.0,
                0.0,
                1.0,
                7,
                ParamFamily::Filter,
            ),
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
            8000.0, 0.36, 0.2, 5000.0, 0.0003, 0.0, 8.0, 3.0, 0.0, 1.0, 0.04, 0.5, 1.0,
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
        algo_count: 1,
        standard_params: OPENHIHAT_STD,
        special_params: &[
            sp_discrete(
                "openhihat_noise_type",
                "Noise Type",
                0.0,
                0.0,
                3.0,
                5,
                ParamFamily::Osc,
            ),
            sp(
                "openhihat_resonance",
                "Resonance",
                2.0,
                0.1,
                10.0,
                6,
                ParamFamily::Osc,
            ),
            sp(
                "openhihat_shimmer",
                "Shimmer",
                0.0,
                0.0,
                1.0,
                7,
                ParamFamily::Filter,
            ),
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
            6000.0, 0.66, 0.3, 8000.0, 0.0003, 0.4, 5.5, 3.0, 0.0, 0.0, 0.05, 0.5, 0.0,
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
                0.3,
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
            196.0, 0.35, 0.7, 600.0, 0.0015, 0.25, 4.0, 3.0, 0.0, 1.0, 0.06, 0.5, 0.0,
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
                0.3,
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
            150.0, 0.3, 0.7, 650.0, 0.0015, 0.2, 4.0, 3.0, 0.0, 1.0, 0.06, 0.5, 0.0,
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
                0.3,
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
            100.0, 0.45, 0.7, 500.0, 0.0015, 0.35, 4.0, 3.0, 0.0, 1.0, 0.06, 0.5, 0.0,
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
        standard_params: CLAP_STD,
        special_params: &[
            sp("clap_echo", "Echo", 0.5, 0.0, 3.0, 0, ParamFamily::Env),
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
            1200.0, 0.03, 1.0, 1000.0, 0.0015, 0.12, 6.0, 3.0, 0.0, 0.0, 0.05, 0.5, 1.0,
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
            8000.0, 1.2, 0.35, 10000.0, 0.002, 1.5, 3.5, 3.0, 0.0, 0.0, 0.05, 0.5, 0.0,
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
            sp_unit(
                "cymbal_shimmer_freq",
                "Shimmer Freq",
                15.0,
                1.0,
                50.0,
                0,
                ParamFamily::Osc,
                " Hz",
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
            6000.0, 2.0, 0.4, 8000.0, 0.002, 2.5, 2.8, 3.0, 0.0, 0.0, 0.05, 0.5, 0.0,
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
            sp("snare606_tone", "Tone", 0.55, 0.0, 1.0, 1, ParamFamily::Osc),
            sp("snare606_snap", "Snap", 0.3, 0.0, 1.0, 2, ParamFamily::Osc),
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
            220.0, 0.08, 0.7, 3000.0, 0.0003, 0.15, 5.0, 3.0, 0.0, 0.0, 0.05, 0.5, 0.0,
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
            sp_unit(
                "bassdrum808_click_tone",
                "Click Tone",
                4000.0,
                100.0,
                8000.0,
                3,
                ParamFamily::Filter,
                " Hz",
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
            sp_discrete(
                "bassdrum808_saturation_pre_filter",
                "Saturation Pre-Filter",
                0.0,
                0.0,
                1.0,
                8,
                ParamFamily::Saturation,
            ),
        ],
        // Minimal: no hold, no filter env, no stereo
        sound_settings_default: [
            50.0, 0.4, 1.0, 3000.0, 0.0015, 0.0, 3.0, 3.0, 0.0, 0.0, 0.05, 0.5, 0.0,
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
            sp("perc1_sweep", "Sweep", 0.5, -1.0, 1.0, 0, ParamFamily::Osc),
            sp(
                "perc1_speed",
                "Speed",
                80.0,
                5.0,
                300.0,
                1,
                ParamFamily::Osc,
            ),
            sp("perc1_bite", "Bite", 0.0, 0.0, 1.0, 2, ParamFamily::Osc),
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
            2000.0, 0.15, 0.6, 6000.0, 0.0005, 0.0, 5.0, 3.0, 0.0, 0.7, 0.03, 0.5, 1.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 13,
        name: "BD606",
        label: "B6",
        full_name: "BD6smp",
        midi_note: 41,
        algo_count: 1,
        standard_params: SMP606_STD,
        special_params: &[
            sp_discrete(
                "bd606_analog_mode",
                "Analog Mode",
                1.0,
                0.0,
                1.0,
                0,
                ParamFamily::Osc,
            ),
            sp_discrete("bd606_sample", "Sample", 1.0, 1.0, 8.0, 1, ParamFamily::Osc),
            sp_discrete(
                "bd606_one_shot",
                "One Shot",
                0.0,
                0.0,
                1.0,
                2,
                ParamFamily::Env,
            ),
            sp(
                "bd606_start_offset",
                "Start",
                0.0,
                0.0,
                1.0,
                3,
                ParamFamily::Osc,
            ),
            sp_unit(
                "bd606_fine_tune", "Pitch Fine",
                0.0,
                -100.0,
                100.0,
                9,
                ParamFamily::Osc,
                " ct",
            ),
            sp(
                "bd606_end",
                "End",
                1.0,
                0.0,
                1.0,
                11,
                ParamFamily::Osc,
            ),
            sp_discrete(
                "bd606_saturation_type",
                "Saturation Type",
                0.0,
                0.0,
                5.0,
                4,
                ParamFamily::Saturation,
            ),
            sp(
                "bd606_saturation_amount",
                "Saturation Amount",
                0.0,
                0.0,
                1.0,
                5,
                ParamFamily::Saturation,
            ),
            sp(
                "bd606_saturation_mix",
                "Saturation Mix",
                1.0,
                0.0,
                1.0,
                6,
                ParamFamily::Saturation,
            ),
            sp(
                "bd606_saturation_output_gain",
                "Saturation Output Gain",
                1.0,
                0.5,
                2.0,
                7,
                ParamFamily::Saturation,
            ),
            sp_discrete(
                "bd606_saturation_pre_filter",
                "Saturation Pre-Filter",
                0.0,
                0.0,
                1.0,
                8,
                ParamFamily::Saturation,
            ),
        ],
        // [freq, decay, vol, filter_freq, attack, release, decay_curve,
        //  release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]
        // (filter_env_decay = fraction of the sample length)
        sound_settings_default: [
            0.0, 0.4, 1.0, 20000.0, 0.002, 0.0, 4.0, 3.0, 0.0, 0.0, 0.15, 1.0, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 14,
        name: "SD606",
        label: "s6",
        full_name: "SD6smp",
        midi_note: 40,
        algo_count: 1,
        standard_params: SMP606_STD,
        special_params: &[
            sp_discrete(
                "sd606_analog_mode",
                "Analog Mode",
                1.0,
                0.0,
                1.0,
                0,
                ParamFamily::Osc,
            ),
            sp_discrete("sd606_sample", "Sample", 1.0, 1.0, 8.0, 1, ParamFamily::Osc),
            sp_discrete(
                "sd606_one_shot",
                "One Shot",
                0.0,
                0.0,
                1.0,
                2,
                ParamFamily::Env,
            ),
            sp(
                "sd606_start_offset",
                "Start",
                0.0,
                0.0,
                1.0,
                3,
                ParamFamily::Osc,
            ),
            sp_unit(
                "sd606_fine_tune", "Pitch Fine",
                0.0,
                -100.0,
                100.0,
                9,
                ParamFamily::Osc,
                " ct",
            ),
            sp(
                "sd606_end",
                "End",
                1.0,
                0.0,
                1.0,
                11,
                ParamFamily::Osc,
            ),
            sp_discrete(
                "sd606_saturation_type",
                "Saturation Type",
                0.0,
                0.0,
                5.0,
                4,
                ParamFamily::Saturation,
            ),
            sp(
                "sd606_saturation_amount",
                "Saturation Amount",
                0.0,
                0.0,
                1.0,
                5,
                ParamFamily::Saturation,
            ),
            sp(
                "sd606_saturation_mix",
                "Saturation Mix",
                1.0,
                0.0,
                1.0,
                6,
                ParamFamily::Saturation,
            ),
            sp(
                "sd606_saturation_output_gain",
                "Saturation Output Gain",
                1.0,
                0.5,
                2.0,
                7,
                ParamFamily::Saturation,
            ),
            sp_discrete(
                "sd606_saturation_pre_filter",
                "Saturation Pre-Filter",
                0.0,
                0.0,
                1.0,
                8,
                ParamFamily::Saturation,
            ),
        ],
        // [freq, decay, vol, filter_freq, attack, release, decay_curve,
        //  release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]
        // (filter_env_decay = fraction of the sample length)
        sound_settings_default: [
            0.0, 0.3, 0.8, 20000.0, 0.002, 0.0, 4.0, 3.0, 0.0, 0.0, 0.15, 1.0, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 15,
        name: "CH606",
        label: "c6",
        full_name: "CH6smp",
        midi_note: 42,
        algo_count: 1,
        standard_params: SMP606_STD,
        special_params: &[
            sp_discrete(
                "ch606_analog_mode",
                "Analog Mode",
                1.0,
                0.0,
                1.0,
                0,
                ParamFamily::Osc,
            ),
            sp_discrete("ch606_sample", "Sample", 1.0, 1.0, 8.0, 1, ParamFamily::Osc),
            sp_discrete(
                "ch606_one_shot",
                "One Shot",
                0.0,
                0.0,
                1.0,
                2,
                ParamFamily::Env,
            ),
            sp(
                "ch606_start_offset",
                "Start",
                0.0,
                0.0,
                1.0,
                3,
                ParamFamily::Osc,
            ),
            sp_unit(
                "ch606_fine_tune", "Pitch Fine",
                0.0,
                -100.0,
                100.0,
                9,
                ParamFamily::Osc,
                " ct",
            ),
            sp(
                "ch606_end",
                "End",
                1.0,
                0.0,
                1.0,
                11,
                ParamFamily::Osc,
            ),
            sp_discrete(
                "ch606_saturation_type",
                "Saturation Type",
                0.0,
                0.0,
                5.0,
                4,
                ParamFamily::Saturation,
            ),
            sp(
                "ch606_saturation_amount",
                "Saturation Amount",
                0.0,
                0.0,
                1.0,
                5,
                ParamFamily::Saturation,
            ),
            sp(
                "ch606_saturation_mix",
                "Saturation Mix",
                1.0,
                0.0,
                1.0,
                6,
                ParamFamily::Saturation,
            ),
            sp(
                "ch606_saturation_output_gain",
                "Saturation Output Gain",
                1.0,
                0.5,
                2.0,
                7,
                ParamFamily::Saturation,
            ),
            sp_discrete(
                "ch606_saturation_pre_filter",
                "Saturation Pre-Filter",
                0.0,
                0.0,
                1.0,
                8,
                ParamFamily::Saturation,
            ),
        ],
        // [freq, decay, vol, filter_freq, attack, release, decay_curve,
        //  release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]
        sound_settings_default: [
            0.0, 0.2, 0.6, 20000.0, 0.001, 0.0, 4.0, 3.0, 0.0, 0.0, 0.15, 1.0, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "LP",
    },
    InstrumentDef {
        index: 16,
        name: "Buzz",
        label: "Bz",
        full_name: "Buzz",
        midi_note: 44,
        algo_count: 2,
        standard_params: BUZZ_STD,
        special_params: &[
            sp_unit("buzz_gate_rate", "Gate Rate", 55.0, 1.0, 500.0, 0, ParamFamily::Env, " Hz"),
            sp("buzz_gate_depth", "Gate Depth", 0.85, 0.0, 1.0, 1, ParamFamily::Env),
            sp("buzz_gate_shape", "Gate Shape", 0.55, 0.0, 1.0, 2, ParamFamily::Env),
            sp("buzz_noise_amount", "Noise", 0.3, 0.0, 1.0, 3, ParamFamily::Osc),
            sp_discrete(
                "buzz_noise_type",
                "Noise Type",
                0.0,
                0.0,
                3.0,
                4,
                ParamFamily::Osc,
            ),
            sp("buzz_pitch_sweep", "Pitch Sweep", 0.3, 0.0, 1.0, 5, ParamFamily::Osc),
            sp_discrete("buzz_wave", "Wave", 0.0, 0.0, 2.0, 11, ParamFamily::Osc),
            sp_unit(
                "buzz_filter_attack",
                "Filter Attack",
                0.0,
                0.0,
                0.5,
                12,
                ParamFamily::Filter,
                " s",
            ),
            sp_unit(
                "buzz_filter_hold",
                "Filter Hold",
                0.0,
                0.0,
                0.5,
                13,
                ParamFamily::Filter,
                " s",
            ),
            sp(
                "buzz_filter_atk_curve",
                "Filter Atk Curve",
                0.0,
                -1.0,
                1.0,
                16,
                ParamFamily::Filter,
            ),
            sp(
                "buzz_filter_curve",
                "Filter Dec Curve",
                0.6,
                -1.0,
                1.0,
                15,
                ParamFamily::Filter,
            ),
            sp_discrete(
                "buzz_filter_type",
                "Filter Type",
                0.0,
                0.0,
                2.0,
                14,
                ParamFamily::Filter,
            ),
            sp_discrete(
                "buzz_saturation_type",
                "Saturation Type",
                0.0,
                0.0,
                5.0,
                6,
                ParamFamily::Saturation,
            ),
            sp(
                "buzz_saturation_amount",
                "Saturation Amount",
                0.0,
                0.0,
                1.0,
                7,
                ParamFamily::Saturation,
            ),
            sp(
                "buzz_saturation_mix",
                "Saturation Mix",
                1.0,
                0.0,
                1.0,
                8,
                ParamFamily::Saturation,
            ),
            sp(
                "buzz_saturation_output_gain",
                "Saturation Output Gain",
                1.0,
                0.5,
                2.0,
                9,
                ParamFamily::Saturation,
            ),
            sp_discrete(
                "buzz_saturation_pre_filter",
                "Saturation Pre-Filter",
                0.0,
                0.0,
                1.0,
                10,
                ParamFamily::Saturation,
            ),
        ],
        // [freq, decay, vol, filter_freq, attack, release, decay_curve,
        //  release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]
        sound_settings_default: [
            200.0, 0.5, 1.3, 1200.0, 0.0005, 0.2, 4.0, 3.0, 0.0, 0.6, 0.12, 0.5, 1.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "",
    },
    InstrumentDef {
        index: 17,
        name: "Sdrex",
        label: "Sx",
        full_name: "SDrex",
        midi_note: 48,
        algo_count: 1,
        standard_params: SDREX_STD,
        special_params: &[
            sp_discrete(
                "sdrex_filter_mod",
                "Modulation",
                0.0,
                0.0,
                1.0,
                17,
                ParamFamily::Modulation,
            ),
            sp_unit(
                "sdrex_flanger_rate",
                "Rate",
                5.7,
                0.1,
                20.0,
                0,
                ParamFamily::Modulation,
                " Hz",
            ),
            sp_unit(
                "sdrex_modulation_fade",
                "Fade-in",
                0.0,
                0.0,
                300.0,
                1,
                ParamFamily::Modulation,
                " ms",
            ),
            sp(
                "sdrex_flanger_depth",
                "Depth",
                1.8,
                0.0,
                3.0,
                2,
                ParamFamily::Modulation,
            ),
            sp(
                "sdrex_flanger_feedback",
                "Feedback",
                0.38,
                0.0,
                0.9,
                3,
                ParamFamily::Modulation,
            ),
            sp(
                "sdrex_flanger_wet",
                "Wet",
                0.32,
                0.0,
                1.0,
                4,
                ParamFamily::Modulation,
            ),
            sp_discrete(
                "sdrex_saturation_type",
                "Saturation Type",
                0.0,
                0.0,
                5.0,
                5,
                ParamFamily::Saturation,
            ),
            sp(
                "sdrex_saturation_amount",
                "Saturation Amount",
                0.0,
                0.0,
                1.0,
                6,
                ParamFamily::Saturation,
            ),
            sp(
                "sdrex_saturation_mix",
                "Saturation Mix",
                1.0,
                0.0,
                1.0,
                7,
                ParamFamily::Saturation,
            ),
            sp(
                "sdrex_saturation_output_gain",
                "Saturation Output Gain",
                1.0,
                0.5,
                2.0,
                8,
                ParamFamily::Saturation,
            ),
            sp_discrete(
                "sdrex_saturation_pre_filter",
                "Saturation Pre-Filter",
                0.0,
                0.0,
                1.0,
                9,
                ParamFamily::Saturation,
            ),
            sp(
                "sdrex_noise_level", "Noise", 0.8, 0.0, 1.0, 10, ParamFamily::Osc,
            ),
            sp_discrete(
                "sdrex_noise_type",
                "Noise Type",
                0.0,
                0.0,
                3.0,
                11,
                ParamFamily::Osc,
            ),
            sp_discrete(
                "sdrex_free_phase",
                "Free Phase",
                0.0,
                0.0,
                1.0,
                12,
                ParamFamily::Modulation,
            ),
            sp_unit(
                "sdrex_filter_attack",
                "Filter Attack",
                0.0,
                0.0,
                0.5,
                13,
                ParamFamily::Filter,
                " s",
            ),
            sp_unit(
                "sdrex_filter_hold",
                "Filter Hold",
                0.0,
                0.0,
                1.0,
                16,
                ParamFamily::Filter,
                " s",
            ),
            sp(
                "sdrex_filter_atk_curve",
                "Filter Atk Curve",
                0.0,
                -1.0,
                1.0,
                14,
                ParamFamily::Filter,
            ),
            sp(
                "sdrex_filter_dec_curve",
                "Filter Dec Curve",
                0.6,
                -1.0,
                1.0,
                15,
                ParamFamily::Filter,
            ),
        ],
        // [freq, decay, vol, filter_freq, attack, release, decay_curve,
        //  release_curve, hold, filter_env_amount, filter_env_decay, analog, stereo]
        sound_settings_default: [
            185.0, 0.15, 0.9, 20000.0, 0.0005, 0.0, 0.0, 0.0, 0.0, 0.0, 0.05, 0.5, 0.0,
        ],
        freq_display_ratio: 1.0,
        filter_type_label: "",
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
        match def.widget {
            ParamWidget::Slider { min, max, .. } => {
                fields.push(MorphableField {
                    field_index: def.field.plock_field_index(),
                    label: def.label,
                    min,
                    max,
                });
            }
            ParamWidget::Checkbox => {
                fields.push(MorphableField {
                    field_index: def.field.plock_field_index(),
                    label: def.label,
                    min: 0.0,
                    max: 1.0,
                });
            }
        }
    }

    let standard_field_indices: std::collections::HashSet<usize> = inst
        .standard_params
        .iter()
        .map(|def| def.field.plock_field_index())
        .collect();

    for def in inst.special_params {
        if !def.continuous {
            continue;
        }
        let field_index = SPECIAL_FIELD_START + def.special_index;
        if field_index == StandardField::Attack.plock_field_index()
            || standard_field_indices.contains(&field_index)
        {
            continue;
        }
        fields.push(MorphableField {
            field_index,
            label: def.label,
            min: def.min,
            max: def.max,
        });
    }

    fields
}

/// Map an incoming MIDI note number to a voice index.
/// Returns `Some(index)` if the note matches one of the instrument's default
/// MIDI notes, `None` otherwise.
pub fn voice_idx_from_midi_note(note: u8) -> Option<usize> {
    INSTRUMENTS.iter().position(|inst| inst.midi_note == note)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_stereo(def: &InstrumentDef) -> bool {
        def.standard_params
            .iter()
            .any(|p| p.field == StandardField::Stereo)
    }

    #[test]
    fn stereo_capable_voices_expose_the_stereo_checkbox() {
        // Snare, HiHat, OpenHiHat, Clap, Ride, Cymbal, Snare606, Perc1,
        // BD606, SD606, CH606 ([168]: two-sample L&R stereo), Buzz.
        for idx in [1usize, 2, 3, 7, 8, 9, 10, 12, 13, 14, 15, 16] {
            assert!(
                has_stereo(&INSTRUMENTS[idx]),
                "{} (idx {idx}) lost its Stereo standard param",
                INSTRUMENTS[idx].name
            );
        }
    }

    #[test]
    fn mono_voices_do_not_expose_the_stereo_checkbox() {
        // Kick, Tom1-3, B8, Sdrex stay mono.
        for idx in [0usize, 4, 5, 6, 11, 17] {
            assert!(
                !has_stereo(&INSTRUMENTS[idx]),
                "{} (idx {idx}) unexpectedly exposes Stereo",
                INSTRUMENTS[idx].name
            );
        }
    }

    #[test]
    fn sdrex_modulation_has_its_own_parameter_family() {
        let sdrex = &INSTRUMENTS[17];
        // The Modulation section is exactly these params ([181] renamed the
        // flanger's Delay into the shared `sdrex_modulation_fade`).
        let modulation: Vec<&str> = sdrex
            .special_params
            .iter()
            .filter(|param| param.family == ParamFamily::Modulation)
            .map(|param| param.name)
            .collect();
        assert_eq!(
            modulation,
            vec![
                "sdrex_filter_mod",
                "sdrex_flanger_rate",
                "sdrex_modulation_fade",
                "sdrex_flanger_depth",
                "sdrex_flanger_feedback",
                "sdrex_flanger_wet",
                "sdrex_free_phase",
            ]
        );
        assert!(sdrex
            .special_params
            .iter()
            .filter(|param| {
                param.name.starts_with("sdrex_filter_") && param.name != "sdrex_filter_mod"
            })
            .all(|param| param.family == ParamFamily::Filter));
        let free_phase = sdrex
            .special_params
            .iter()
            .find(|param| param.name == "sdrex_free_phase")
            .unwrap();
        assert_eq!(free_phase.family, ParamFamily::Modulation);
        let filter_mod = sdrex
            .special_params
            .iter()
            .find(|param| param.name == "sdrex_filter_mod")
            .unwrap();
        assert_eq!(filter_mod.family, ParamFamily::Modulation);
        assert!(!filter_mod.continuous);
    }

    /// [182] Special params measuring a physical quantity must declare their unit
    /// — `SpecialParamDef` had no unit field at all, so every one of them used to
    /// render as a bare number while standard params showed " s" / " Hz".
    ///
    /// Two guards: an exact snapshot of who carries a unit (so a dimensionless
    /// "amount" never gains a bogus one), and a keyword rule so the NEXT
    /// frequency or time parameter added cannot silently forget it.
    #[test]
    fn physical_special_params_declare_their_unit() {
        let with_unit: Vec<(&str, &str)> = INSTRUMENTS
            .iter()
            .flat_map(|inst| inst.special_params.iter())
            .filter_map(|param| param.unit.map(|unit| (param.name, unit)))
            .collect();
        assert_eq!(
            with_unit,
            vec![
                ("cymbal_shimmer_freq", " Hz"),
                ("bassdrum808_click_tone", " Hz"),
                ("bd606_fine_tune", " ct"),
                ("sd606_fine_tune", " ct"),
                ("ch606_fine_tune", " ct"),
                ("buzz_gate_rate", " Hz"),
                ("buzz_filter_attack", " s"),
                ("buzz_filter_hold", " s"),
                ("sdrex_flanger_rate", " Hz"),
                ("sdrex_modulation_fade", " ms"),
                ("sdrex_filter_attack", " s"),
                ("sdrex_filter_hold", " s"),
            ]
        );

        // Anything named like a frequency or a duration needs one. `_atk_curve`
        // and `snare606_tone` (a 0..1 blend) deliberately do not match.
        const NEEDS_UNIT: [&str; 6] = [
            "_rate", "_freq", "_attack", "_hold", "_fade", "_fine_tune",
        ];
        for inst in INSTRUMENTS.iter() {
            for param in inst.special_params {
                if NEEDS_UNIT.iter().any(|kw| param.name.contains(kw)) {
                    assert!(
                        param.unit.is_some(),
                        "{} measures a physical quantity but declares no unit",
                        param.name
                    );
                }
            }
        }
    }

    /// [181] The decay sliders of the two kicks and the clap are capped where the
    /// sound actually stops changing — 5 s of convex decay was mostly inaudible
    /// tail, so most of the slider travel did nothing. The Cymbal, which shares
    /// the clap's parameter shape, must KEEP its long range.
    #[test]
    fn kick_bd808_and_clap_decay_caps_leave_the_cymbal_alone() {
        let decay_max = |name: &str| {
            let inst = INSTRUMENTS
                .iter()
                .find(|inst| inst.name == name)
                .unwrap_or_else(|| panic!("no instrument named {name}"));
            let def = inst
                .standard_params
                .iter()
                .find(|param| param.field == StandardField::Decay)
                .unwrap();
            let ParamWidget::Slider { max, .. } = def.widget else {
                panic!("Decay should be a slider on {name}");
            };
            max
        };
        assert_eq!(decay_max("Kick"), 2.0);
        assert_eq!(decay_max("BassDrum808"), 2.0);
        assert_eq!(decay_max("Clap"), 1.5);
        assert_eq!(decay_max("Cymbal"), 5.0, "the cymbal needs its long tail");
    }

    /// Decays reach 1.5 s; the holds are deliberately capped at 1 s ([181] — 2 s
    /// of hold was unusable slider travel).
    #[test]
    fn sdrex_envelope_ranges_are_capped_where_the_user_wants_them() {
        let sdrex = &INSTRUMENTS[17];
        for field in [
            StandardField::Decay,
            StandardField::Hold,
            StandardField::FilterEnvDecay,
        ] {
            let def = sdrex
                .standard_params
                .iter()
                .find(|param| param.field == field)
                .unwrap();
            let ParamWidget::Slider { max, .. } = def.widget else {
                panic!("{field:?} should be a slider");
            };
            let expected = if field == StandardField::Hold { 1.0 } else { 1.5 };
            assert_eq!(max, expected, "unexpected maximum for {field:?}");
        }
        let filter_hold = sdrex
            .special_params
            .iter()
            .find(|param| param.name == "sdrex_filter_hold")
            .unwrap();
        assert_eq!(filter_hold.max, 1.0);
        assert_eq!(filter_hold.family, ParamFamily::Filter);
    }
}
