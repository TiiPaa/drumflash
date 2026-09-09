//! Typed settings for the AC606 voices (ported analogcode engines).
//!
//! One shared struct for all six kinds — the engines only consume a handful
//! of standard params (frequency, attack on BD, decay, volume, analog) plus
//! `snap` (special[0], SD only). Unused standard fields are kept so the
//! settings round-trip stays total.

use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcVoiceSettings {
    pub frequency: f32,
    pub decay: f32,
    pub volume: f32,
    pub filter_freq: f32,
    pub attack: f32,
    pub release: f32,
    pub decay_curve: f32,
    pub release_curve: f32,
    pub hold: f32,
    pub filter_env_amount: f32,
    pub filter_env_decay: f32,
    pub analog: f32,
    pub stereo: f32,
    /// Kind-specific exploration params ([196]) + the saturation pack —
    /// indices are per-kind, see `ac_voice.rs`.
    pub special: [f32; 32],
    pub algo: u8,
}

impl From<VoiceSettings> for AcVoiceSettings {
    fn from(v: VoiceSettings) -> Self {
        Self {
            frequency: v.frequency,
            decay: v.decay,
            volume: v.volume,
            filter_freq: v.filter_freq,
            attack: v.attack,
            release: v.release,
            decay_curve: v.decay_curve,
            release_curve: v.release_curve,
            hold: v.hold,
            filter_env_amount: v.filter_env_amount,
            filter_env_decay: v.filter_env_decay,
            analog: v.analog,
            stereo: v.stereo,
            special: v.special,
            algo: v.algo,
        }
    }
}

impl From<AcVoiceSettings> for VoiceSettings {
    fn from(s: AcVoiceSettings) -> Self {
        Self {
            frequency: s.frequency,
            decay: s.decay,
            volume: s.volume,
            filter_freq: s.filter_freq,
            attack: s.attack,
            release: s.release,
            decay_curve: s.decay_curve,
            release_curve: s.release_curve,
            hold: s.hold,
            filter_env_amount: s.filter_env_amount,
            filter_env_decay: s.filter_env_decay,
            analog: s.analog,
            stereo: s.stereo,
            algo: s.algo,
            special: s.special,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AcVoiceSettings;
    crate::settings_roundtrip_test!(bd6ac_settings_roundtrip, bd6ac, AcVoiceSettings);
    crate::settings_roundtrip_test!(sd6ac_settings_roundtrip, sd6ac, AcVoiceSettings);
    crate::settings_roundtrip_test!(hh6ac_settings_roundtrip, hh6ac, AcVoiceSettings);
    crate::settings_roundtrip_test!(oh6ac_settings_roundtrip, oh6ac, AcVoiceSettings);
    crate::settings_roundtrip_test!(cl6ac_settings_roundtrip, cl6ac, AcVoiceSettings);
    crate::settings_roundtrip_test!(tm6ac_settings_roundtrip, tm6ac, AcVoiceSettings);
}
