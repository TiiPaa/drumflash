use crate::synthesis::VoiceSettings;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClapSettings {
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
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for ClapSettings {
    fn from(v: VoiceSettings) -> Self {
        Self {
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
            saturation_type: v.special[1] as u8,
            saturation_amount: v.special[2],
            saturation_mix: v.special[3],
            saturation_output_gain: v.special[4],
            saturation_pre_filter: v.special[5],
            algo: v.algo,
        }
    }
}

impl From<ClapSettings> for VoiceSettings {
    fn from(c: ClapSettings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = c.echo;
        special[1] = c.saturation_type as f32;
        special[2] = c.saturation_amount;
        special[3] = c.saturation_mix;
        special[4] = c.saturation_output_gain;
        special[5] = c.saturation_pre_filter;
        Self {
            frequency: 1000.0,
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
    crate::settings_roundtrip_test!(clap_settings_roundtrip, clap, ClapSettings, skip_frequency);
}
