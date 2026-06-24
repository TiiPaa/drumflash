use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RideSettings {
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

impl From<VoiceSettings> for RideSettings {
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

impl From<RideSettings> for VoiceSettings {
    fn from(r: RideSettings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = r.saturation_type as f32;
        special[1] = r.saturation_amount;
        special[2] = r.saturation_mix;
        special[3] = r.saturation_output_gain;
        special[4] = r.saturation_pre_filter;
        Self {
            frequency: r.frequency,
            attack: r.attack,
            decay: r.decay,
            decay_curve: r.decay_curve,
            release: r.release,
            release_curve: r.release_curve,
            volume: r.volume,
            filter_freq: r.filter_freq,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            hold: 0.0,
            analog: r.analog,
            stereo: r.stereo,
            algo: r.algo,
            special,
        }
    }
}
