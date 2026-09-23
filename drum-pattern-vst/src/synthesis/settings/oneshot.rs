use crate::synthesis::VoiceSettings;

/// One-Shot ([243]) — plays the lane's own sample file, start to finish.
///
/// No embedded content: the sample is the lane's [228] user file, so the
/// `texture` special is only an infra marker (it keeps the file alive through
/// the lane-texture rules) — the voice always reads the lane's file. The
/// layout is contiguous (a new instrument has no legacy to honour).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OneShotSettings {
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
    /// Infra marker only ([228] lane-file rules key on a `_texture` special).
    pub texture: f32,
    /// 1.0 = the file is read backwards.
    pub reverse: f32,
    /// Where playback starts, as a fraction of the file (0..1). Forward skips
    /// the first `offset` of the file; Reverse counts it from the END (it
    /// starts at `1 - offset` and reads down to the file start), so the
    /// marker is the start of the sound in both directions.
    pub offset: f32,
    /// Fine tuning in cents, added to the semitone Pitch.
    pub pitch_fine: f32,
    /// A-H-D pitch envelope: depth in semitones (bipolar), times as FRACTIONS
    /// of the played region's heard duration, each ramp shaped by its own
    /// bipolar curve.
    pub pitch_env: f32,
    pub pitch_env_attack: f32,
    pub pitch_env_hold: f32,
    pub pitch_env_decay: f32,
    pub pitch_env_atk_curve: f32,
    pub pitch_env_dec_curve: f32,
    /// 0 = low-pass, 1 = high-pass, 2 = band-pass (same mapping as Rift).
    pub filter_type: f32,
    /// Filter Q.
    pub resonance: f32,
    /// A-H-D filter envelope, same shape as Rift but times as FRACTIONS of
    /// the played region; decay from the standard `filter_env_decay`.
    pub filter_attack: f32,
    pub filter_hold: f32,
    pub filter_atk_curve: f32,
    pub filter_dec_curve: f32,
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for OneShotSettings {
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
            texture: v.special[0],
            reverse: v.special[1],
            offset: v.special[20],
            pitch_fine: v.special[2],
            pitch_env: v.special[3],
            pitch_env_attack: v.special[4],
            pitch_env_hold: v.special[5],
            pitch_env_decay: v.special[6],
            pitch_env_atk_curve: v.special[7],
            pitch_env_dec_curve: v.special[8],
            filter_type: v.special[9],
            resonance: v.special[10],
            filter_attack: v.special[11],
            filter_hold: v.special[12],
            filter_atk_curve: v.special[13],
            filter_dec_curve: v.special[14],
            saturation_type: v.special[15] as u8,
            saturation_amount: v.special[16],
            saturation_mix: v.special[17],
            saturation_output_gain: v.special[18],
            saturation_pre_filter: v.special[19],
            algo: v.algo,
        }
    }
}

impl From<OneShotSettings> for VoiceSettings {
    fn from(s: OneShotSettings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = s.texture;
        special[1] = s.reverse;
        special[2] = s.pitch_fine;
        special[3] = s.pitch_env;
        special[4] = s.pitch_env_attack;
        special[5] = s.pitch_env_hold;
        special[6] = s.pitch_env_decay;
        special[7] = s.pitch_env_atk_curve;
        special[8] = s.pitch_env_dec_curve;
        special[9] = s.filter_type;
        special[10] = s.resonance;
        special[11] = s.filter_attack;
        special[12] = s.filter_hold;
        special[13] = s.filter_atk_curve;
        special[14] = s.filter_dec_curve;
        special[15] = s.saturation_type as f32;
        special[16] = s.saturation_amount;
        special[17] = s.saturation_mix;
        special[18] = s.saturation_output_gain;
        special[19] = s.saturation_pre_filter;
        special[20] = s.offset;
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
    crate::settings_roundtrip_test!(oneshot_settings_roundtrip, oneshot, OneShotSettings);
}
