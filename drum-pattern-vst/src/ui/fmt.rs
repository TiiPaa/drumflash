//! Value / note / frequency formatting helpers shared by the plock menus.

pub fn freq_to_note(freq: f32) -> f32 {
    if freq <= 0.0 || !freq.is_finite() {
        return 0.0;
    }
    69.0 + 12.0 * (freq / 440.0).log2()
}

pub fn note_to_freq(note: f32) -> f32 {
    if !note.is_finite() {
        return 440.0;
    }
    440.0 * 2.0f32.powf((note - 69.0) / 12.0)
}

/// [182] `unit` is the param's display unit, so the plock menu shows the same
/// thing as the Sound Panel row it overrides ("0.50 s", not "0.50").
pub fn format_value_for_plock(
    field: crate::instrument_registry::StandardField,
    value: f32,
    min: f32,
    max: f32,
    unit: Option<&str>,
) -> String {
    format!(
        "{}{}",
        format_plock_number(field, value, min, max),
        unit.unwrap_or("")
    )
}

fn format_plock_number(
    field: crate::instrument_registry::StandardField,
    value: f32,
    min: f32,
    max: f32,
) -> String {
    match field {
        crate::instrument_registry::StandardField::Volume => format!("{:.2}", value),
        crate::instrument_registry::StandardField::Analog
        | crate::instrument_registry::StandardField::Stereo => format!("{:.2}", value),
        crate::instrument_registry::StandardField::Decay
        | crate::instrument_registry::StandardField::Release
        | crate::instrument_registry::StandardField::Attack
        | crate::instrument_registry::StandardField::Hold
        | crate::instrument_registry::StandardField::FilterEnvDecay => format!("{:.2}", value),
        crate::instrument_registry::StandardField::DecayCurve
        | crate::instrument_registry::StandardField::ReleaseCurve
        | crate::instrument_registry::StandardField::FilterEnvAmount => format!("{:.2}", value),
        crate::instrument_registry::StandardField::Freq
        | crate::instrument_registry::StandardField::FilterFreq => {
            let range = max - min;
            if range >= 1000.0 || max >= 1000.0 {
                format!("{:.1}", value)
            } else {
                format!("{:.2}", value)
            }
        }
    }
}

pub fn format_value_for_plock_special(
    value: f32,
    min: f32,
    max: f32,
    unit: Option<&str>,
) -> String {
    format!(
        "{}{}",
        format_plock_special_number(value, min, max),
        unit.unwrap_or("")
    )
}

fn format_plock_special_number(value: f32, min: f32, max: f32) -> String {
    let range = max - min;
    if range >= 1000.0 || max >= 1000.0 {
        format!("{:.1}", value)
    } else if range <= 1.0 {
        format!("{:.2}", value)
    } else {
        format!("{:.2}", value)
    }
}

pub fn note_name(note: f32) -> String {
    let note = note.round() as i32;
    let note = note.clamp(0, 127);
    let octave = (note / 12) - 1;
    let note_idx = note % 12;
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", names[note_idx as usize], octave)
}
