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
    /// FM shimmer modulation frequency in Hz.
    pub shimmer_freq: f32,
    /// FM shimmer depth (0.0 = no shimmer, 1.0 = full shimmer).
    pub shimmer_amount: f32,
    /// Noise colour: 0=white, 1=pink, 2=brown, 3=blue.
    pub noise_type: u8,
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
            shimmer_freq: v.special[0],
            shimmer_amount: v.special[2],
            noise_type: v.special[1] as u8,
            saturation_type: v.special[3] as u8,
            saturation_amount: v.special[4],
            saturation_mix: v.special[5],
            saturation_output_gain: v.special[6],
            saturation_pre_filter: v.special[7],
        }
    }
}

impl From<CymbalSettings> for VoiceSettings {
    fn from(c: CymbalSettings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = c.shimmer_freq;
        special[1] = c.noise_type as f32;
        special[2] = c.shimmer_amount;
        special[3] = c.saturation_type as f32;
        special[4] = c.saturation_amount;
        special[5] = c.saturation_mix;
        special[6] = c.saturation_output_gain;
        special[7] = c.saturation_pre_filter;
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::settings_roundtrip_test!(cymbal_settings_roundtrip, cymbal, CymbalSettings);
}
