use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HiHatSettings {
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

impl From<VoiceSettings> for HiHatSettings {
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

impl From<HiHatSettings> for VoiceSettings {
    fn from(h: HiHatSettings) -> Self {
        Self {
            frequency: h.frequency,
            attack: h.attack,
            decay: h.decay,
            decay_curve: h.decay_curve,
            release: h.release,
            release_curve: h.release_curve,
            volume: h.volume,
            filter_freq: h.filter_freq,
            filter_env_amount: h.filter_env_amount,
            filter_env_decay: h.filter_env_decay,
            hold: h.hold,
            analog: h.analog,
            stereo: h.stereo,
            algo: h.algo,
            special: [0.0; 8],
        }
    }
}
