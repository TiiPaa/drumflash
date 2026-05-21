use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snare606Settings {
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
    pub resonance: f32,
    pub tone: f32,
    pub snap: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for Snare606Settings {
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
            resonance: v.special[0],
            tone: v.special[1],
            snap: v.special[2],
            algo: v.algo,
        }
    }
}

impl From<Snare606Settings> for VoiceSettings {
    fn from(s: Snare606Settings) -> Self {
        let mut special = [0.0f32; 8];
        special[0] = s.resonance;
        special[1] = s.tone;
        special[2] = s.snap;
        Self {
            frequency: s.frequency,
            attack: s.attack,
            decay: s.decay,
            decay_curve: s.decay_curve,
            release: s.release,
            release_curve: s.release_curve,
            volume: s.volume,
            filter_freq: s.filter_freq,
            filter_env_amount: s.filter_env_amount,
            filter_env_decay: s.filter_env_decay,
            hold: s.hold,
            analog: s.analog,
            stereo: s.stereo,
            algo: s.algo,
            special,
        }
    }
}
