use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Perc1Settings {
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
    pub sweep: f32,
    pub speed: f32,
    pub bite: f32,
    pub width: f32,
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for Perc1Settings {
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
            sweep: v.special[0],
            speed: v.special[1],
            bite: v.special[2],
            width: v.special[3],
            saturation_type: v.special[4] as u8,
            saturation_amount: v.special[5],
            saturation_mix: v.special[6],
            saturation_output_gain: v.special[7],
            saturation_pre_filter: v.special[8],
            algo: v.algo,
        }
    }
}

impl From<Perc1Settings> for VoiceSettings {
    fn from(p: Perc1Settings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = p.sweep;
        special[1] = p.speed;
        special[2] = p.bite;
        special[3] = p.width;
        special[4] = p.saturation_type as f32;
        special[5] = p.saturation_amount;
        special[6] = p.saturation_mix;
        special[7] = p.saturation_output_gain;
        special[8] = p.saturation_pre_filter;
        Self {
            frequency: p.frequency,
            attack: p.attack,
            decay: p.decay,
            decay_curve: p.decay_curve,
            release: p.release,
            release_curve: p.release_curve,
            volume: p.volume,
            filter_freq: p.filter_freq,
            filter_env_amount: p.filter_env_amount,
            filter_env_decay: p.filter_env_decay,
            hold: p.hold,
            analog: p.analog,
            stereo: p.stereo,
            algo: p.algo,
            special,
        }
    }
}
