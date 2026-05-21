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
            algo: v.algo,
        }
    }
}

impl From<KickSettings> for VoiceSettings {
    fn from(k: KickSettings) -> Self {
        let mut special = [0.0f32; 8];
        special[0] = k.click_level;
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

    #[test]
    fn kick_settings_roundtrip_preserves_all_fields() {
        let v = VoiceSettings::kick();
        let k = KickSettings::from(v);
        let v2 = VoiceSettings::from(k);

        assert_eq!(v.frequency, v2.frequency);
        assert_eq!(v.attack, v2.attack);
        assert_eq!(v.decay, v2.decay);
        assert_eq!(v.decay_curve, v2.decay_curve);
        assert_eq!(v.release, v2.release);
        assert_eq!(v.release_curve, v2.release_curve);
        assert_eq!(v.volume, v2.volume);
        assert_eq!(v.filter_freq, v2.filter_freq);
        assert_eq!(v.filter_env_amount, v2.filter_env_amount);
        assert_eq!(v.filter_env_decay, v2.filter_env_decay);
        assert_eq!(v.analog, v2.analog);
        assert_eq!(v.algo, v2.algo);
        assert_eq!(v.special[0], v2.special[0]);
    }
}
