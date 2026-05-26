use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CymbalSettings {
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
    pub algo: u8,
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
}

impl From<VoiceSettings> for CymbalSettings {
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
            algo: v.algo,
            saturation_type: v.special[0] as u8,
            saturation_amount: v.special[1],
            saturation_mix: v.special[2],
            saturation_output_gain: v.special[3],
            saturation_pre_filter: v.special[4],
        }
    }
}

impl From<CymbalSettings> for VoiceSettings {
    fn from(c: CymbalSettings) -> Self {
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
            special: [
                c.saturation_type as f32,
                c.saturation_amount,
                c.saturation_mix,
                c.saturation_output_gain,
                c.saturation_pre_filter,
                0.0,
                0.0,
                0.0,
            ],
        }
    }
}
