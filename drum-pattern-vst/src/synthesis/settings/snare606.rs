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
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
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
            saturation_type: v.special[3] as u8,
            saturation_amount: v.special[4],
            saturation_mix: v.special[5],
            saturation_output_gain: v.special[6],
            saturation_pre_filter: v.special[7],
            algo: v.algo,
        }
    }
}

impl From<Snare606Settings> for VoiceSettings {
    fn from(s: Snare606Settings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = s.resonance;
        special[1] = s.tone;
        special[2] = s.snap;
        special[3] = s.saturation_type as f32;
        special[4] = s.saturation_amount;
        special[5] = s.saturation_mix;
        special[6] = s.saturation_output_gain;
        special[7] = s.saturation_pre_filter;
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::settings_roundtrip_test!(snare606_settings_roundtrip, snare606, Snare606Settings);
}
