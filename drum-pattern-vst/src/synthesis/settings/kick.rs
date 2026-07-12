//! Typed settings for the Kick voice.

use crate::synthesis::VoiceSettings;

/// All parameters that affect Kick synthesis, with named fields.
///
/// Standard fields are copied from `VoiceSettings`; the special field
/// `click_level` replaces `special[0]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KickSettings {
    pub frequency: f32,
    pub attack: f32,
    pub decay: f32,
    pub decay_curve: f32,
    pub release: f32,
    pub release_curve: f32,
    pub volume: f32,
    pub filter_freq: f32,
    pub filter_env_amount: f32,
    pub filter_env_decay: f32,
    pub analog: f32,
    pub click_level: f32,
    pub click_type: u8,
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
    pub algo: u8,
}

impl KickSettings {
    /// Defaults used by tests and cold-start initialization.
    pub fn default_at(_sr: f32) -> Self {
        let v = VoiceSettings::kick();
        Self::from(v)
    }
}

impl From<VoiceSettings> for KickSettings {
    fn from(v: VoiceSettings) -> Self {
        Self {
            frequency: v.frequency,
            attack: v.attack,
            decay: v.decay,
            decay_curve: v.decay_curve,
            release: v.release,
            release_curve: v.release_curve,
            volume: v.volume,
            filter_freq: v.filter_freq,
            filter_env_amount: v.filter_env_amount,
            filter_env_decay: v.filter_env_decay,
            analog: v.analog,
            click_level: v.special[0],
            click_type: v.special[6] as u8,
            saturation_type: v.special[1] as u8,
            saturation_amount: v.special[2],
            saturation_mix: v.special[3],
            saturation_output_gain: v.special[4],
            saturation_pre_filter: v.special[5],
            algo: v.algo,
        }
    }
}

impl From<KickSettings> for VoiceSettings {
    fn from(k: KickSettings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = k.click_level;
        special[6] = k.click_type as f32;
        special[1] = k.saturation_type as f32;
        special[2] = k.saturation_amount;
        special[3] = k.saturation_mix;
        special[4] = k.saturation_output_gain;
        special[5] = k.saturation_pre_filter;
        Self {
            frequency: k.frequency,
            attack: k.attack,
            decay: k.decay,
            decay_curve: k.decay_curve,
            release: k.release,
            release_curve: k.release_curve,
            volume: k.volume,
            filter_freq: k.filter_freq,
            analog: k.analog,
            stereo: 0.0,
            hold: 0.0,
            filter_env_amount: k.filter_env_amount,
            filter_env_decay: k.filter_env_decay,
            algo: k.algo,
            special,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::settings_roundtrip_test!(kick_settings_roundtrip, kick, KickSettings);
}
