use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpenHiHatSettings {
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
    pub hold: f32,
    pub analog: f32,
    pub stereo: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for OpenHiHatSettings {
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
            hold: v.hold,
            analog: v.analog,
            stereo: v.stereo,
            algo: v.algo,
        }
    }
}

impl From<OpenHiHatSettings> for VoiceSettings {
    fn from(o: OpenHiHatSettings) -> Self {
        Self {
            frequency: o.frequency,
            attack: o.attack,
            decay: o.decay,
            decay_curve: o.decay_curve,
            release: o.release,
            release_curve: o.release_curve,
            volume: o.volume,
            filter_freq: o.filter_freq,
            filter_env_amount: o.filter_env_amount,
            filter_env_decay: o.filter_env_decay,
            hold: o.hold,
            analog: o.analog,
            stereo: o.stereo,
            algo: o.algo,
            special: [0.0; 8],
        }
    }
}
