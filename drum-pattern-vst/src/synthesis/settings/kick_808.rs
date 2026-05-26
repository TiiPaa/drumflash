use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kick808Settings {
    pub frequency: f32,
    pub attack: f32,
    pub decay: f32,
    pub decay_curve: f32,
    pub release: f32,
    pub release_curve: f32,
    pub volume: f32,
    pub filter_freq: f32,
    pub analog: f32,
    pub accent: f32,
    pub snap: f32,
    pub pitch_drop: f32,
    pub click_tone: f32,
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for Kick808Settings {
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
            accent: v.special[0],
            snap: v.special[1],
            pitch_drop: v.special[2],
            click_tone: v.special[3],
            saturation_type: v.special[4] as u8,
            saturation_amount: v.special[5],
            saturation_mix: v.special[6],
            saturation_output_gain: v.special[7],
            saturation_pre_filter: 0.0,
            algo: v.algo,
        }
    }
}

impl From<Kick808Settings> for VoiceSettings {
    fn from(k: Kick808Settings) -> Self {
        let mut special = [0.0f32; 8];
        special[0] = k.accent;
        special[1] = k.snap;
        special[2] = k.pitch_drop;
        special[3] = k.click_tone;
        special[4] = k.saturation_type as f32;
        special[5] = k.saturation_amount;
        special[6] = k.saturation_mix;
        special[7] = k.saturation_output_gain;
        Self {
            frequency: k.frequency,
            attack: k.attack,
            decay: k.decay,
            decay_curve: k.decay_curve,
            release: k.release,
            release_curve: k.release_curve,
            volume: k.volume,
            filter_freq: k.filter_freq,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            hold: 0.0,
            analog: k.analog,
            stereo: 0.0,
            algo: k.algo,
            special,
        }
    }
}
