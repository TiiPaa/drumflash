use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bd606Settings {
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
    /// 1.0 = analog mode (random multisample per trigger, no immediate
    /// repeat); 0.0 = always the same sample (see `sample_index`).
    pub analog_mode: f32,
    /// Fixed sample selection when `analog_mode` is off: 1..=8.
    pub sample_index: f32,
    /// 1.0 = play the sample to its end, bypassing the amp envelope.
    pub one_shot: f32,
    /// Skip into the sample, as a fraction of the sample length (0..1).
    pub start_offset: f32,
    /// Fine tune in cents (-100..+100), added to the semitone pitch.
    pub fine_tune: f32,
    /// End of playback, as a fraction of the sample length (0..1). Default 1.
    pub end: f32,
    /// 0 = legacy Hz pitch, 1 = relative semitone pitch.
    pub pitch_format_version: f32,
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for Bd606Settings {
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
            analog_mode: v.special[0],
            sample_index: v.special[1],
            one_shot: v.special[2],
            start_offset: v.special[3],
            fine_tune: v.special[9],
            end: v.special[11],
            pitch_format_version: v.special[10],
            saturation_type: v.special[4] as u8,
            saturation_amount: v.special[5],
            saturation_mix: v.special[6],
            saturation_output_gain: v.special[7],
            saturation_pre_filter: v.special[8],
            algo: v.algo,
        }
    }
}

impl From<Bd606Settings> for VoiceSettings {
    fn from(b: Bd606Settings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = b.analog_mode;
        special[1] = b.sample_index;
        special[2] = b.one_shot;
        special[3] = b.start_offset;
        special[9] = b.fine_tune;
        special[11] = b.end;
        special[10] = b.pitch_format_version;
        special[4] = b.saturation_type as f32;
        special[5] = b.saturation_amount;
        special[6] = b.saturation_mix;
        special[7] = b.saturation_output_gain;
        special[8] = b.saturation_pre_filter;
        Self {
            frequency: b.frequency,
            attack: b.attack,
            decay: b.decay,
            decay_curve: b.decay_curve,
            release: b.release,
            release_curve: b.release_curve,
            volume: b.volume,
            filter_freq: b.filter_freq,
            filter_env_amount: b.filter_env_amount,
            filter_env_decay: b.filter_env_decay,
            hold: b.hold,
            analog: b.analog,
            stereo: b.stereo,
            algo: b.algo,
            special,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::settings_roundtrip_test!(bd606_settings_roundtrip, bd606, Bd606Settings);
}
