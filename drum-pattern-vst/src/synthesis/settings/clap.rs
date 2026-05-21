use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClapSettings {
    pub frequency: f32,
    pub attack: f32,
    pub decay: f32,
    pub decay_curve: f32,
    pub release: f32,
    pub release_curve: f32,
    pub volume: f32,
    pub filter_freq: f32,
    pub analog: f32,
    pub stereo: f32,
    pub echo: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for ClapSettings {
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
            analog: v.analog,
            stereo: v.stereo,
            echo: v.special[0],
            algo: v.algo,
        }
    }
}

impl From<ClapSettings> for VoiceSettings {
    fn from(c: ClapSettings) -> Self {
        let mut special = [0.0f32; 8];
        special[0] = c.echo;
        Self {
            frequency: c.frequency,
            attack: c.attack,
            decay: c.decay,
            decay_curve: c.decay_curve,
            release: c.release,
            release_curve: c.release_curve,
            volume: c.volume,
            filter_freq: c.filter_freq,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            hold: 0.0,
            analog: c.analog,
            stereo: c.stereo,
            algo: c.algo,
            special,
        }
    }
}
