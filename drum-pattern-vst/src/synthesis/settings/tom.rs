use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TomSettings {
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
    pub stick_attack: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for TomSettings {
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
            stick_attack: v.special[0],
            algo: v.algo,
        }
    }
}

impl From<TomSettings> for VoiceSettings {
    fn from(t: TomSettings) -> Self {
        let mut special = [0.0f32; 8];
        special[0] = t.stick_attack;
        Self {
            frequency: t.frequency,
            attack: t.attack,
            decay: t.decay,
            decay_curve: t.decay_curve,
            release: t.release,
            release_curve: t.release_curve,
            volume: t.volume,
            filter_freq: t.filter_freq,
            filter_env_amount: t.filter_env_amount,
            filter_env_decay: t.filter_env_decay,
            hold: 0.0,
            analog: t.analog,
            stereo: 0.0,
            algo: t.algo,
            special,
        }
    }
}
