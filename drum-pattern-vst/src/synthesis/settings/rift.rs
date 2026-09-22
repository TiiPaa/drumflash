use crate::synthesis::VoiceSettings;

/// Rift ([221]) — a slice lifted out of a long texture.
///
/// The star is `offset`: because it is a plain continuous special parameter it
/// is p-lockable per step and morphable across a fusion for free, which is what
/// turns one texture into sixteen different sounds over a page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiftSettings {
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
    /// Which embedded texture: the index into the registry's `options` list.
    pub texture: f32,
    /// Where the slice starts, as a fraction of the texture (0..1).
    pub offset: f32,
    /// Random deviation around `offset`, re-rolled at every trigger. 0 = the
    /// cell always sounds the same.
    pub wander: f32,
    /// 1.0 = the slice is read backwards.
    pub reverse: f32,
    /// 1.0 = the grain repeats under the envelope; 0.0 = the slice is played
    /// once and the envelope alone decides how long the sound lasts.
    pub loop_mode: f32,
    /// Grain length, 0..1 mapped exponentially onto 5 ms .. 4 s. Only bites
    /// while `loop_mode` is on.
    pub grain: f32,
    /// Fade at both ends of the grain, as a FRACTION of its length (not a
    /// duration - hence the name, which keeps it out of the registry's
    /// "declare your unit" rule).
    pub grain_shape: f32,
    /// 0 = low-pass, 1 = high-pass, 2 = band-pass.
    pub filter_type: f32,
    /// Filter Q.
    pub resonance: f32,
    /// A-H-D filter envelope, same shape as Buzz and SDrex: attack and hold in
    /// seconds, decay from the standard `filter_env_decay`, each ramp shaped by
    /// its own bipolar curve.
    pub filter_attack: f32,
    pub filter_hold: f32,
    pub filter_atk_curve: f32,
    pub filter_dec_curve: f32,
    /// Fine tuning in cents, added to the semitone Pitch.
    pub pitch_fine: f32,
    /// Pitch envelope depth in semitones, bipolar, decaying over
    /// `pitch_env_time`.
    pub pitch_env: f32,
    pub pitch_env_time: f32,
    /// Offset step added at every trigger, as a fraction of the texture — the
    /// sequential mode.
    pub advance: f32,
    /// Shape and phase policy shared by both LFOs.
    pub lfo_shape: f32,
    pub lfo_free_phase: f32,
    /// Pitch LFO: rate in Hz, depth in semitones.
    pub pitch_lfo_rate: f32,
    pub pitch_lfo_depth: f32,
    /// Filter LFO: rate in Hz, depth in octaves around the swept cutoff.
    pub filter_lfo_rate: f32,
    pub filter_lfo_depth: f32,
    /// Lo-fi pair (build 3): bit-depth reduction and sample-rate division,
    /// both 0..1 amounts, applied to the slice before the amp envelope so the
    /// tail never gates.
    pub crush: f32,
    pub decimate: f32,
    /// Right channel reads the texture this far AHEAD of the left (0..1 of
    /// half a second, squared). 0 = mono.
    pub stereo_spread: f32,
    pub saturation_type: u8,
    pub saturation_amount: f32,
    pub saturation_mix: f32,
    pub saturation_output_gain: f32,
    pub saturation_pre_filter: f32,
    pub algo: u8,
}

impl From<VoiceSettings> for RiftSettings {
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
            offset: v.special[1],
            wander: v.special[2],
            grain: v.special[3],
            grain_shape: v.special[4],
            loop_mode: v.special[12],
            reverse: v.special[27],
            filter_type: v.special[5],
            resonance: v.special[6],
            filter_attack: v.special[13],
            filter_hold: v.special[14],
            filter_atk_curve: v.special[15],
            filter_dec_curve: v.special[16],
            pitch_fine: v.special[17],
            pitch_env: v.special[18],
            pitch_env_time: v.special[19],
            advance: v.special[20],
            lfo_shape: v.special[21],
            lfo_free_phase: v.special[22],
            pitch_lfo_rate: v.special[23],
            pitch_lfo_depth: v.special[24],
            filter_lfo_rate: v.special[25],
            filter_lfo_depth: v.special[26],
            crush: v.special[28],
            decimate: v.special[29],
            stereo_spread: v.special[30],
            saturation_type: v.special[7] as u8,
            saturation_amount: v.special[8],
            saturation_mix: v.special[9],
            saturation_output_gain: v.special[10],
            saturation_pre_filter: v.special[11],
            algo: v.algo,
        }
    }
}

impl From<RiftSettings> for VoiceSettings {
    fn from(r: RiftSettings) -> Self {
        let mut special = [0.0f32; 32];
        special[0] = r.texture;
        special[1] = r.offset;
        special[2] = r.wander;
        special[3] = r.grain;
        special[4] = r.grain_shape;
        special[12] = r.loop_mode;
        special[27] = r.reverse;
        special[5] = r.filter_type;
        special[6] = r.resonance;
        special[13] = r.filter_attack;
        special[14] = r.filter_hold;
        special[15] = r.filter_atk_curve;
        special[16] = r.filter_dec_curve;
        special[17] = r.pitch_fine;
        special[18] = r.pitch_env;
        special[19] = r.pitch_env_time;
        special[20] = r.advance;
        special[21] = r.lfo_shape;
        special[22] = r.lfo_free_phase;
        special[23] = r.pitch_lfo_rate;
        special[24] = r.pitch_lfo_depth;
        special[25] = r.filter_lfo_rate;
        special[26] = r.filter_lfo_depth;
        special[28] = r.crush;
        special[29] = r.decimate;
        special[30] = r.stereo_spread;
        special[7] = r.saturation_type as f32;
        special[8] = r.saturation_amount;
        special[9] = r.saturation_mix;
        special[10] = r.saturation_output_gain;
        special[11] = r.saturation_pre_filter;
        Self {
            frequency: r.frequency,
            attack: r.attack,
            decay: r.decay,
            decay_curve: r.decay_curve,
            release: r.release,
            release_curve: r.release_curve,
            volume: r.volume,
            filter_freq: r.filter_freq,
            filter_env_amount: r.filter_env_amount,
            filter_env_decay: r.filter_env_decay,
            hold: r.hold,
            analog: r.analog,
            stereo: r.stereo,
            algo: r.algo,
            special,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::settings_roundtrip_test!(rift_settings_roundtrip, rift, RiftSettings);
}
