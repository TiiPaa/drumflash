//! Audio synthesis module for drum sounds

mod clap;
mod cymbal;
mod dsp;
mod hihat;
mod kick;
mod kick_808;
mod open_hihat;
mod perc1;
mod ride;
mod saturation;
mod settings;
mod snare;
mod snare606;
mod special_params;
mod tom;

// `algos_for` is consumed by the editor (`ui.rs`) but not by the standalone
// binary — allow the unused-import warning so both build configurations stay
// clean.
#[allow(unused_imports)]
pub use special_params::{algos_for, AlgoDef};

pub use clap::ClapVoice;
pub use cymbal::CymbalVoice;
pub use hihat::HiHatVoice;
pub use kick::KickVoice;
pub use kick_808::Kick808Voice;
pub use open_hihat::OpenHiHatVoice;
pub use perc1::Perc1Voice;
pub use ride::RideVoice;
pub use settings::clap::ClapSettings;
pub use settings::cymbal::CymbalSettings;
pub use settings::hihat::HiHatSettings;
pub use settings::kick::KickSettings;
pub use settings::kick_808::Kick808Settings;
pub use settings::open_hihat::OpenHiHatSettings;
pub use settings::perc1::Perc1Settings;
pub use settings::ride::RideSettings;
pub use settings::snare::SnareSettings;
pub use settings::snare606::Snare606Settings;
pub use settings::tom::TomSettings;
pub use snare::SnareVoice;
pub use snare606::Snare606Voice;
pub use tom::TomVoice;

/// Drum voice types matching the original web app
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrumVoice {
    Kick = 0,
    Snare = 1,
    HiHat = 2,
    OpenHiHat = 3,
    Tom1 = 4,
    Tom2 = 5,
    Tom3 = 6,
    Clap = 7,
    Ride = 8,
    Cymbal = 9,
    Snare606 = 10,
    BassDrum808 = 11,
    Perc1 = 12,
}

#[allow(dead_code)]
impl DrumVoice {
    pub const COUNT: usize = 13;

    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Kick),
            1 => Some(Self::Snare),
            2 => Some(Self::HiHat),
            3 => Some(Self::OpenHiHat),
            4 => Some(Self::Tom1),
            5 => Some(Self::Tom2),
            6 => Some(Self::Tom3),
            7 => Some(Self::Clap),
            8 => Some(Self::Ride),
            9 => Some(Self::Cymbal),
            10 => Some(Self::Snare606),
            11 => Some(Self::BassDrum808),
            12 => Some(Self::Perc1),
            _ => None,
        }
    }

    pub fn midi_note(&self) -> u8 {
        crate::instrument_registry::INSTRUMENTS[*self as usize].midi_note
    }

    pub fn name(&self) -> &'static str {
        crate::instrument_registry::INSTRUMENTS[*self as usize].name
    }

    pub fn label(&self) -> &'static str {
        crate::instrument_registry::INSTRUMENTS[*self as usize].label
    }

    /// Steepness of the filter envelope decay stage for voices that use a fixed
    /// filter curve. `None` means the curve is dynamic (`decay_curve`).
    pub fn filter_env_curve(self) -> Option<f32> {
        match self {
            DrumVoice::Kick => Some(KickVoice::FILTER_ENV_CURVE),
            DrumVoice::Snare => Some(SnareVoice::FILTER_ENV_CURVE),
            DrumVoice::HiHat => Some(HiHatVoice::FILTER_ENV_CURVE),
            DrumVoice::Tom1 | DrumVoice::Tom2 | DrumVoice::Tom3 => Some(TomVoice::FILTER_ENV_CURVE),
            DrumVoice::Snare606 => Some(Snare606Voice::FILTER_ENV_CURVE),
            // Perc1 uses the amplitude decay_curve for its filter envelope.
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceSettings {
    pub frequency: f32,
    pub decay: f32,
    pub volume: f32,
    pub filter_freq: f32,
    /// Attack ramp length in seconds for the amplitude envelope. This is both
    /// the audible attack shape and the anti-click retrigger ramp.
    pub attack: f32,
    /// Slow release tail length in seconds. The amplitude envelope is bi-stage:
    /// `amp = max(decay_env, release_env)`. The decay_env starts at 1.0 and drops
    /// fast with `decay`; the release_env starts at a fixed shelf (~30 % of peak)
    /// and decays slowly with `release`, taking over once the decay phase falls
    /// below the shelf. Set `release` to 0 to get a single-stage decay.
    pub release: f32,
    /// Steepness of the decay stage (typical range 2..10). Low values give a
    /// near-linear early fall, high values a steep punchy drop.
    pub decay_curve: f32,
    /// Steepness of the release stage (typical range 2..10). Lower values give
    /// a long flat tail, higher values a quick falloff.
    pub release_curve: f32,
    /// Hold time in seconds. After the attack ramp the envelope stays at its
    /// peak for `hold` seconds before the decay starts. Used by snare and
    /// hi-hat voices to add a short sustain phase. 0 = no hold.
    pub hold: f32,
    /// Filter envelope amount (0..1). Scales the per-voice maximum depth so
    /// that 0 disables the filter envelope and 1 gives the full effect.
    pub filter_env_amount: f32,
    /// Filter envelope decay time in seconds (typically 1 ms .. 200 ms).
    pub filter_env_decay: f32,
    /// Analog drift amount (0..1). 1.0 = full analog behavior (phase continuous,
    /// pitch envelope persistent, filter state retained). 0.0 = digital stable
    /// (phase reset, pitch envelope reset, filter state reset on trigger).
    pub analog: f32,
    /// Stereo width (0..1). 1.0 = stereo (independent L/R noise generators).
    /// 0.0 = mono. Only affects noise-based voices.
    pub stereo: f32,
    /// Synthesis algorithm index (interpreted per instrument).
    pub algo: u8,
    /// Special parameters, indexed per instrument convention.
    pub special: [f32; 32],
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            frequency: 60.0,
            decay: 0.5,
            volume: 0.8,
            filter_freq: 100.0,
            attack: 0.0015,
            release: 0.0,
            decay_curve: 5.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 0.5,
            stereo: 0.0,
            algo: 0,
            special: [0.0; 32],
        }
    }
}

impl VoiceSettings {
    pub fn kick() -> Self {
        Self {
            frequency: 60.0,
            decay: 0.5,
            volume: 0.8,
            filter_freq: 30.0,
            attack: 0.0015,
            release: 0.5,
            decay_curve: 5.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.05,
            analog: 0.5,
            stereo: 0.0,
            algo: 0,
            special: [
                0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn snare() -> Self {
        Self {
            frequency: 200.0,
            decay: 0.47,
            volume: 0.6,
            filter_freq: 200.0,
            attack: 0.0003,
            release: 0.2,
            decay_curve: 5.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.03,
            analog: 0.5,
            stereo: 1.0,
            algo: 0,
            special: [
                0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn hihat() -> Self {
        Self {
            frequency: 8000.0,
            decay: 0.36,
            volume: 0.3,
            filter_freq: 5000.0,
            attack: 0.0003,
            release: 0.0,
            decay_curve: 8.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.04,
            analog: 0.5,
            stereo: 1.0,
            algo: 0,
            special: [
                0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn open_hihat() -> Self {
        Self {
            frequency: 6000.0,
            decay: 0.66,
            volume: 0.4,
            filter_freq: 8000.0,
            attack: 0.0003,
            release: 0.4,
            decay_curve: 5.5,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 0.5,
            stereo: 0.0,
            algo: 0,
            special: [
                0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn tom1() -> Self {
        Self {
            frequency: 196.0,
            decay: 0.35,
            volume: 0.7,
            filter_freq: 600.0,
            attack: 0.0015,
            release: 0.25,
            decay_curve: 4.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.06,
            analog: 0.5,
            stereo: 0.0,
            algo: 0,
            special: [
                0.3, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn tom2() -> Self {
        Self {
            frequency: 150.0,
            decay: 0.3,
            volume: 0.7,
            filter_freq: 650.0,
            attack: 0.0015,
            release: 0.2,
            decay_curve: 4.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.06,
            analog: 0.5,
            stereo: 0.0,
            algo: 0,
            special: [
                0.3, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn tom3() -> Self {
        Self {
            frequency: 100.0,
            decay: 0.45,
            volume: 0.7,
            filter_freq: 500.0,
            attack: 0.0015,
            release: 0.35,
            decay_curve: 4.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.06,
            analog: 0.5,
            stereo: 0.0,
            algo: 0,
            special: [
                0.3, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn clap() -> Self {
        Self {
            frequency: 1200.0,
            decay: 0.03,
            volume: 0.7,
            filter_freq: 1000.0,
            attack: 0.0015,
            release: 0.12,
            decay_curve: 6.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 0.5,
            stereo: 1.0,
            algo: 0,
            special: [
                0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn ride() -> Self {
        Self {
            frequency: 8000.0,
            decay: 1.2,
            volume: 0.35,
            filter_freq: 10000.0,
            attack: 0.002,
            release: 1.5,
            decay_curve: 3.5,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 0.5,
            stereo: 1.0,
            algo: 0,
            special: [
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn cymbal() -> Self {
        Self {
            frequency: 6000.0,
            decay: 2.0,
            volume: 0.4,
            filter_freq: 8000.0,
            attack: 0.002,
            release: 2.5,
            decay_curve: 2.8,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 0.5,
            stereo: 1.0,
            algo: 0,
            special: [
                15.0, 0.0, 0.15, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn snare606() -> Self {
        Self {
            frequency: 220.0,
            decay: 0.08,
            volume: 0.7,
            filter_freq: 3000.0,
            attack: 0.0003,
            release: 0.15,
            decay_curve: 5.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 0.5,
            stereo: 0.0,
            algo: 0,
            // special[0] = Resonance (Q)
            // special[1] = Tone (body vs wires balance)
            // special[2] = Snap (crispness of wires layer)
            special: [
                4.5, 0.55, 0.3, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn kick808() -> Self {
        Self {
            frequency: 50.0,
            decay: 0.4,
            volume: 0.9,
            filter_freq: 3000.0,
            attack: 0.0015,
            release: 0.0,
            decay_curve: 3.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.0,
            filter_env_decay: 0.05,
            analog: 0.5,
            stereo: 0.0,
            algo: 0,
            // special[0] = Accent (click level)
            // special[1] = Snap (pitch sweep depth)
            // special[2] = Pitch Drop amount
            // Defaults at 0 so the user hears the difference when raising sliders.
            special: [
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }

    pub fn perc1() -> Self {
        Self {
            frequency: 2000.0,
            decay: 0.15,
            volume: 0.6,
            filter_freq: 6000.0,
            attack: 0.0005,
            release: 0.0,
            decay_curve: 5.0,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 0.7,
            filter_env_decay: 0.03,
            analog: 0.5,
            stereo: 1.0,
            algo: 0,
            // special[0] = Sweep amount (-1..1)
            // special[1] = Sweep speed (ms)
            // special[2] = Bite (FM amount)
            // special[3] = Width (stereo + slap delay)
            special: [
                0.5, 80.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        }
    }
}

pub trait Voice: Send + Sync {
    fn trigger(&mut self);
    /// Hard machine-gun retrigger: envelope restarts from zero. Default
    /// falls back to regular trigger for voices that don't override.
    fn trigger_hard(&mut self) {
        self.trigger();
    }
    fn process_sample(&mut self) -> f32;
    /// Stereo version. Default returns duplicated mono.
    fn process_sample_stereo(&mut self) -> (f32, f32) {
        let m = self.process_sample();
        (m, m)
    }
    #[allow(dead_code)]
    fn is_active(&self) -> bool;
    fn reset(&mut self);
    #[allow(dead_code)]
    fn set_settings(&mut self, settings: VoiceSettings);

    /// Set synthesis algorithm by index.
    fn set_algo(&mut self, algo: u8);
    /// Set a special parameter by index (0..7).
    #[allow(dead_code)]
    fn set_special_param(&mut self, index: usize, value: f32);
}

/// Concrete enum wrapping all drum voice types.
/// Eliminates dynamic dispatch from the audio path.
pub enum DrumVoiceKind {
    Kick(KickVoice),
    Snare(SnareVoice),
    HiHat(HiHatVoice),
    OpenHiHat(OpenHiHatVoice),
    Tom(TomVoice),
    Clap(ClapVoice),
    Ride(RideVoice),
    Cymbal(CymbalVoice),
    Snare606(Snare606Voice),
    BassDrum808(Kick808Voice),
    Perc1(Perc1Voice),
}

impl Voice for DrumVoiceKind {
    fn trigger(&mut self) {
        match self {
            DrumVoiceKind::Kick(v) => v.trigger(),
            DrumVoiceKind::Snare(v) => v.trigger(),
            DrumVoiceKind::HiHat(v) => v.trigger(),
            DrumVoiceKind::OpenHiHat(v) => v.trigger(),
            DrumVoiceKind::Tom(v) => v.trigger(),
            DrumVoiceKind::Clap(v) => v.trigger(),
            DrumVoiceKind::Ride(v) => v.trigger(),
            DrumVoiceKind::Cymbal(v) => v.trigger(),
            DrumVoiceKind::Snare606(v) => v.trigger(),
            DrumVoiceKind::BassDrum808(v) => v.trigger(),
            DrumVoiceKind::Perc1(v) => v.trigger(),
        }
    }

    fn trigger_hard(&mut self) {
        match self {
            DrumVoiceKind::Kick(v) => v.trigger_hard(),
            DrumVoiceKind::Snare(v) => v.trigger_hard(),
            DrumVoiceKind::HiHat(v) => v.trigger_hard(),
            DrumVoiceKind::OpenHiHat(v) => v.trigger_hard(),
            DrumVoiceKind::Tom(v) => v.trigger_hard(),
            DrumVoiceKind::Clap(v) => v.trigger_hard(),
            DrumVoiceKind::Ride(v) => v.trigger_hard(),
            DrumVoiceKind::Cymbal(v) => v.trigger_hard(),
            DrumVoiceKind::Snare606(v) => v.trigger_hard(),
            DrumVoiceKind::BassDrum808(v) => v.trigger_hard(),
            DrumVoiceKind::Perc1(v) => v.trigger_hard(),
        }
    }

    fn process_sample(&mut self) -> f32 {
        match self {
            DrumVoiceKind::Kick(v) => v.process_sample(),
            DrumVoiceKind::Snare(v) => v.process_sample(),
            DrumVoiceKind::HiHat(v) => v.process_sample(),
            DrumVoiceKind::OpenHiHat(v) => v.process_sample(),
            DrumVoiceKind::Tom(v) => v.process_sample(),
            DrumVoiceKind::Clap(v) => v.process_sample(),
            DrumVoiceKind::Ride(v) => v.process_sample(),
            DrumVoiceKind::Cymbal(v) => v.process_sample(),
            DrumVoiceKind::Snare606(v) => v.process_sample(),
            DrumVoiceKind::BassDrum808(v) => v.process_sample(),
            DrumVoiceKind::Perc1(v) => v.process_sample(),
        }
    }

    fn process_sample_stereo(&mut self) -> (f32, f32) {
        match self {
            DrumVoiceKind::Kick(v) => v.process_sample_stereo(),
            DrumVoiceKind::Snare(v) => v.process_sample_stereo(),
            DrumVoiceKind::HiHat(v) => v.process_sample_stereo(),
            DrumVoiceKind::OpenHiHat(v) => v.process_sample_stereo(),
            DrumVoiceKind::Tom(v) => v.process_sample_stereo(),
            DrumVoiceKind::Clap(v) => v.process_sample_stereo(),
            DrumVoiceKind::Ride(v) => v.process_sample_stereo(),
            DrumVoiceKind::Cymbal(v) => v.process_sample_stereo(),
            DrumVoiceKind::Snare606(v) => v.process_sample_stereo(),
            DrumVoiceKind::BassDrum808(v) => v.process_sample_stereo(),
            DrumVoiceKind::Perc1(v) => v.process_sample_stereo(),
        }
    }

    fn is_active(&self) -> bool {
        match self {
            DrumVoiceKind::Kick(v) => v.is_active(),
            DrumVoiceKind::Snare(v) => v.is_active(),
            DrumVoiceKind::HiHat(v) => v.is_active(),
            DrumVoiceKind::OpenHiHat(v) => v.is_active(),
            DrumVoiceKind::Tom(v) => v.is_active(),
            DrumVoiceKind::Clap(v) => v.is_active(),
            DrumVoiceKind::Ride(v) => v.is_active(),
            DrumVoiceKind::Cymbal(v) => v.is_active(),
            DrumVoiceKind::Snare606(v) => v.is_active(),
            DrumVoiceKind::BassDrum808(v) => v.is_active(),
            DrumVoiceKind::Perc1(v) => v.is_active(),
        }
    }

    fn reset(&mut self) {
        match self {
            DrumVoiceKind::Kick(v) => v.reset(),
            DrumVoiceKind::Snare(v) => v.reset(),
            DrumVoiceKind::HiHat(v) => v.reset(),
            DrumVoiceKind::OpenHiHat(v) => v.reset(),
            DrumVoiceKind::Tom(v) => v.reset(),
            DrumVoiceKind::Clap(v) => v.reset(),
            DrumVoiceKind::Ride(v) => v.reset(),
            DrumVoiceKind::Cymbal(v) => v.reset(),
            DrumVoiceKind::Snare606(v) => v.reset(),
            DrumVoiceKind::BassDrum808(v) => v.reset(),
            DrumVoiceKind::Perc1(v) => v.reset(),
        }
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        match self {
            DrumVoiceKind::Kick(v) => v.set_settings(settings),
            DrumVoiceKind::Snare(v) => v.set_settings(settings),
            DrumVoiceKind::HiHat(v) => v.set_settings(settings),
            DrumVoiceKind::OpenHiHat(v) => v.set_settings(settings),
            DrumVoiceKind::Tom(v) => v.set_settings(settings),
            DrumVoiceKind::Clap(v) => v.set_settings(settings),
            DrumVoiceKind::Ride(v) => v.set_settings(settings),
            DrumVoiceKind::Cymbal(v) => v.set_settings(settings),
            DrumVoiceKind::Snare606(v) => v.set_settings(settings),
            DrumVoiceKind::BassDrum808(v) => v.set_settings(settings),
            DrumVoiceKind::Perc1(v) => v.set_settings(settings),
        }
    }

    fn set_algo(&mut self, algo: u8) {
        match self {
            DrumVoiceKind::Kick(v) => v.set_algo(algo),
            DrumVoiceKind::Snare(v) => v.set_algo(algo),
            DrumVoiceKind::HiHat(v) => v.set_algo(algo),
            DrumVoiceKind::OpenHiHat(v) => v.set_algo(algo),
            DrumVoiceKind::Tom(v) => v.set_algo(algo),
            DrumVoiceKind::Clap(v) => v.set_algo(algo),
            DrumVoiceKind::Ride(v) => v.set_algo(algo),
            DrumVoiceKind::Cymbal(v) => v.set_algo(algo),
            DrumVoiceKind::Snare606(v) => v.set_algo(algo),
            DrumVoiceKind::BassDrum808(v) => v.set_algo(algo),
            DrumVoiceKind::Perc1(v) => v.set_algo(algo),
        }
    }

    fn set_special_param(&mut self, index: usize, value: f32) {
        match self {
            DrumVoiceKind::Kick(v) => v.set_special_param(index, value),
            DrumVoiceKind::Snare(v) => v.set_special_param(index, value),
            DrumVoiceKind::HiHat(v) => v.set_special_param(index, value),
            DrumVoiceKind::OpenHiHat(v) => v.set_special_param(index, value),
            DrumVoiceKind::Tom(v) => v.set_special_param(index, value),
            DrumVoiceKind::Clap(v) => v.set_special_param(index, value),
            DrumVoiceKind::Ride(v) => v.set_special_param(index, value),
            DrumVoiceKind::Cymbal(v) => v.set_special_param(index, value),
            DrumVoiceKind::Snare606(v) => v.set_special_param(index, value),
            DrumVoiceKind::BassDrum808(v) => v.set_special_param(index, value),
            DrumVoiceKind::Perc1(v) => v.set_special_param(index, value),
        }
    }
}

fn create_voice_for_kind(
    kind: crate::track::TrackInstrumentKind,
    sample_rate: f32,
) -> DrumVoiceKind {
    use crate::track::TrackInstrumentKind as K;
    match kind {
        K::Kick => DrumVoiceKind::Kick(KickVoice::new(
            sample_rate,
            KickSettings::from(VoiceSettings::kick()),
        )),
        K::Snare => DrumVoiceKind::Snare(SnareVoice::new(
            sample_rate,
            SnareSettings::from(VoiceSettings::snare()),
        )),
        K::HiHat => DrumVoiceKind::HiHat(HiHatVoice::new(
            sample_rate,
            HiHatSettings::from(VoiceSettings::hihat()),
        )),
        K::OpenHiHat => DrumVoiceKind::OpenHiHat(OpenHiHatVoice::new(
            sample_rate,
            OpenHiHatSettings::from(VoiceSettings::open_hihat()),
        )),
        K::Tom => DrumVoiceKind::Tom(TomVoice::new(
            sample_rate,
            TomSettings::from(VoiceSettings::tom1()),
        )),
        K::Clap => DrumVoiceKind::Clap(ClapVoice::new(
            sample_rate,
            ClapSettings::from(VoiceSettings::clap()),
        )),
        K::Ride => DrumVoiceKind::Ride(RideVoice::new(
            sample_rate,
            RideSettings::from(VoiceSettings::ride()),
        )),
        K::Cymbal => DrumVoiceKind::Cymbal(CymbalVoice::new(
            sample_rate,
            CymbalSettings::from(VoiceSettings::cymbal()),
        )),
        K::Snare606 => DrumVoiceKind::Snare606(Snare606Voice::new(
            sample_rate,
            Snare606Settings::from(VoiceSettings::snare606()),
        )),
        K::BassDrum808 => DrumVoiceKind::BassDrum808(Kick808Voice::new(
            sample_rate,
            Kick808Settings::from(VoiceSettings::kick808()),
        )),
        K::Perc1 => DrumVoiceKind::Perc1(Perc1Voice::new(
            sample_rate,
            Perc1Settings::from(VoiceSettings::perc1()),
        )),
    }
}

pub struct DrumSynthesizer {
    voices: Box<[Option<Box<DrumVoiceKind>>; crate::track::MAX_TRACKS]>,
    sample_rate: f32,
    velocities: [f32; crate::track::MAX_TRACKS],
    /// One-pole smoothers on each voice's velocity, absorbing gain jumps when
    /// retriggering a voice while its tail is still ringing.
    velocity_smoothers: [dsp::OnePoleSmoother; crate::track::MAX_TRACKS],
}

const VELOCITY_SMOOTH_MS: f32 = 1.5;

impl DrumSynthesizer {
    pub fn new() -> Self {
        Self {
            voices: Box::new(std::array::from_fn(|_| None)),
            sample_rate: 44100.0,
            velocities: [1.0; crate::track::MAX_TRACKS],
            velocity_smoothers: std::array::from_fn(|_| {
                dsp::OnePoleSmoother::new(44100.0, VELOCITY_SMOOTH_MS, 1.0)
            }),
        }
    }

    #[allow(dead_code)]
    pub fn initialize(&mut self, sample_rate: f32) {
        let legacy_layout = crate::track::TrackLayoutState::from_legacy_13();
        self.initialize_with_layout(sample_rate, &legacy_layout);
    }

    pub fn initialize_with_layout(
        &mut self,
        sample_rate: f32,
        layout: &crate::track::TrackLayoutState,
    ) {
        self.sample_rate = sample_rate;
        for smoother in self.velocity_smoothers.iter_mut() {
            *smoother = dsp::OnePoleSmoother::new(sample_rate, VELOCITY_SMOOTH_MS, 1.0);
        }

        for (i, slot) in layout.slots.iter().enumerate() {
            self.voices[i] = if slot.active {
                Some(Box::new(create_voice_for_kind(slot.kind, sample_rate)))
            } else {
                None
            };
        }
    }
    pub fn trigger(&mut self, slot_idx: usize, velocity: f32) {
        if let Some(Some(voice)) = self.voices.get_mut(slot_idx) {
            voice.trigger();
            self.velocities[slot_idx] = velocity;
        }
    }

    pub fn trigger_hard(&mut self, slot_idx: usize, velocity: f32) {
        if let Some(Some(voice)) = self.voices.get_mut(slot_idx) {
            voice.trigger_hard();
            self.velocities[slot_idx] = velocity;
        }
    }

    #[allow(dead_code)]
    pub fn process_sample(&mut self, output: &mut f32) {
        let mut mixed = 0.0f32;
        for voice in self.voices.iter_mut().flatten() {
            mixed += voice.process_sample();
        }
        *output = mixed;
    }

    #[allow(dead_code)]
    pub fn process_voice_samples(&mut self, outputs: &mut [f32; crate::track::MAX_TRACKS]) {
        for (i, (voice, output)) in self.voices.iter_mut().zip(outputs.iter_mut()).enumerate() {
            if let Some(voice) = voice {
                let vel = self.velocity_smoothers[i].process(self.velocities[i]);
                *output = voice.process_sample() * vel;
            } else {
                *output = 0.0;
            }
        }
    }

    pub fn process_voice_samples_stereo(
        &mut self,
        outputs: &mut [[f32; 2]; crate::track::MAX_TRACKS],
    ) {
        for (i, (voice, output)) in self.voices.iter_mut().zip(outputs.iter_mut()).enumerate() {
            if let Some(voice) = voice {
                let vel = self.velocity_smoothers[i].process(self.velocities[i]);
                let (l, r) = voice.process_sample_stereo();
                output[0] = l * vel;
                output[1] = r * vel;
            } else {
                output[0] = 0.0;
                output[1] = 0.0;
            }
        }
    }

    pub fn reset(&mut self) {
        for voice in self.voices.iter_mut().flatten() {
            voice.reset();
        }
    }

    #[allow(dead_code)]
    pub fn set_voice_settings(&mut self, slot_idx: usize, settings: VoiceSettings) {
        if let Some(Some(voice)) = self.voices.get_mut(slot_idx) {
            voice.set_settings(settings);
        }
    }

    pub fn set_algo(&mut self, slot_idx: usize, algo: u8) {
        if let Some(Some(voice)) = self.voices.get_mut(slot_idx) {
            voice.set_algo(algo);
        }
    }

    pub fn reset_voice(&mut self, slot_idx: usize) {
        if let Some(Some(voice)) = self.voices.get_mut(slot_idx) {
            voice.reset();
        }
    }

    pub fn reinitialize_slot(&mut self, slot_idx: usize, kind: crate::track::TrackInstrumentKind) {
        if slot_idx >= crate::track::MAX_TRACKS {
            return;
        }
        let new_voice = create_voice_for_kind(kind, self.sample_rate);
        if let Some(existing) = self.voices[slot_idx].as_mut() {
            **existing = new_voice;
        } else {
            self.voices[slot_idx] = Some(Box::new(new_voice));
        }
        self.velocity_smoothers[slot_idx] =
            dsp::OnePoleSmoother::new(self.sample_rate, VELOCITY_SMOOTH_MS, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: sum absolute output of a single voice after triggering it.
    fn sum_voice_output(synth: &mut DrumSynthesizer, voice_idx: usize, samples: usize) -> f32 {
        let mut sum = 0.0f32;
        let mut outputs = [[0.0f32; 2]; crate::track::MAX_TRACKS];
        for _ in 0..samples {
            synth.process_voice_samples_stereo(&mut outputs);
            sum += outputs[voice_idx][0].abs() + outputs[voice_idx][1].abs();
        }
        sum
    }

    #[test]
    fn bassdrum808_produces_sound_after_cymbal_settings_change() {
        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let b8_idx = DrumVoice::BassDrum808 as usize;
        let cy_idx = DrumVoice::Cymbal as usize;

        // 1. Trigger B8 and verify it produces sound
        synth.trigger(b8_idx, 1.0);
        let out1 = sum_voice_output(&mut synth, b8_idx, 100);
        assert!(
            out1 > 0.0,
            "B8 should produce sound on first trigger: {}",
            out1
        );

        // 2. Change Cymbal settings while B8 is potentially still ringing
        let mut cymbal_settings = VoiceSettings::cymbal();
        cymbal_settings.decay = 1.0;
        cymbal_settings.volume = 0.5;
        synth.set_voice_settings(DrumVoice::Cymbal as usize, cymbal_settings);

        // 3. Trigger B8 again and verify it STILL produces sound
        synth.trigger(b8_idx, 1.0);
        let out2 = sum_voice_output(&mut synth, b8_idx, 100);
        assert!(
            out2 > 0.0,
            "B8 should still produce sound after Cymbal set_voice_settings: {}",
            out2
        );
    }

    #[test]
    fn bassdrum808_produces_sound_after_all_voice_settings_change() {
        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let b8_idx = DrumVoice::BassDrum808 as usize;

        // 1. Trigger B8 and verify it produces sound
        synth.trigger(b8_idx, 1.0);
        let out1 = sum_voice_output(&mut synth, b8_idx, 100);
        assert!(
            out1 > 0.0,
            "B8 should produce sound on first trigger: {}",
            out1
        );

        // 2. Call set_voice_settings on ALL voices (like lib.rs does when sound_settings_state.version changes)
        let all_settings = [
            (DrumVoice::Kick, VoiceSettings::kick()),
            (DrumVoice::Snare, VoiceSettings::snare()),
            (DrumVoice::HiHat, VoiceSettings::hihat()),
            (DrumVoice::OpenHiHat, VoiceSettings::open_hihat()),
            (DrumVoice::Tom1, VoiceSettings::tom1()),
            (DrumVoice::Tom2, VoiceSettings::tom2()),
            (DrumVoice::Tom3, VoiceSettings::tom3()),
            (DrumVoice::Clap, VoiceSettings::clap()),
            (DrumVoice::Ride, VoiceSettings::ride()),
            (DrumVoice::Cymbal, VoiceSettings::cymbal()),
            (DrumVoice::Snare606, VoiceSettings::snare606()),
            (DrumVoice::BassDrum808, VoiceSettings::kick808()),
            (DrumVoice::Perc1, VoiceSettings::perc1()),
        ];

        for (voice, settings) in all_settings.iter() {
            synth.set_voice_settings(*voice as usize, *settings);
        }

        // 3. Trigger B8 again and verify it STILL produces sound
        synth.trigger(b8_idx, 1.0);
        let out2 = sum_voice_output(&mut synth, b8_idx, 100);
        assert!(
            out2 > 0.0,
            "B8 should still produce sound after all set_voice_settings: {}",
            out2
        );
    }

    #[test]
    fn bassdrum808_does_not_go_silent_when_cymbal_triggered_and_settings_changed() {
        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let b8_idx = DrumVoice::BassDrum808 as usize;
        let cy_idx = DrumVoice::Cymbal as usize;

        // Trigger Cymbal first
        synth.trigger(cy_idx, 1.0);

        // Trigger B8 while Cymbal is ringing
        synth.trigger(b8_idx, 1.0);
        let out1 = sum_voice_output(&mut synth, b8_idx, 50);
        assert!(
            out1 > 0.0,
            "B8 should produce sound when triggered alongside Cymbal: {}",
            out1
        );

        // Now modify Cymbal settings while both are active
        let mut cymbal_settings = VoiceSettings::cymbal();
        cymbal_settings.decay = 0.5;
        cymbal_settings.filter_freq = 6000.0;
        synth.set_voice_settings(DrumVoice::Cymbal as usize, cymbal_settings);

        // Trigger B8 again
        synth.trigger(b8_idx, 1.0);
        let out2 = sum_voice_output(&mut synth, b8_idx, 100);
        assert!(
            out2 > 0.0,
            "B8 should still produce sound after Cymbal settings changed while active: {}",
            out2
        );
    }

    /// Helper: checks whether any sample in the output is NaN.
    fn contains_nan(synth: &mut DrumSynthesizer, voice_idx: usize, samples: usize) -> bool {
        let mut outputs = [[0.0f32; 2]; crate::track::MAX_TRACKS];
        for _ in 0..samples {
            synth.process_voice_samples_stereo(&mut outputs);
            if outputs[voice_idx][0].is_nan() || outputs[voice_idx][1].is_nan() {
                return true;
            }
        }
        false
    }

    #[test]
    fn bassdrum808_never_produces_nan_after_cymbal_or_all_settings_changed() {
        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let b8_idx = DrumVoice::BassDrum808 as usize;

        // Trigger B8
        synth.trigger(b8_idx, 1.0);
        assert!(
            !contains_nan(&mut synth, b8_idx, 10),
            "B8 should not produce NaN initially"
        );

        // Change Cymbal with extreme values
        let mut cy_settings = VoiceSettings::cymbal();
        cy_settings.attack = 0.0;
        cy_settings.decay = 0.001;
        cy_settings.release = 0.001;
        cy_settings.decay_curve = 0.1;
        cy_settings.release_curve = 0.1;
        synth.set_voice_settings(DrumVoice::Cymbal as usize, cy_settings);

        // Trigger B8 again
        synth.trigger(b8_idx, 1.0);
        assert!(
            !contains_nan(&mut synth, b8_idx, 200),
            "B8 should not produce NaN after Cymbal extreme settings"
        );

        // Now change ALL voices with edge-case values while B8 is active
        let edge = VoiceSettings {
            frequency: 20.0,
            decay: 0.001,
            volume: 1.5,
            filter_freq: 20000.0,
            attack: 0.0,
            release: 0.001,
            decay_curve: 0.1,
            release_curve: 0.1,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.001,
            analog: 0.0,
            stereo: 1.0,
            algo: 1,
            special: [1.0; 32],
        };

        for voice in [
            DrumVoice::Kick,
            DrumVoice::Snare,
            DrumVoice::HiHat,
            DrumVoice::OpenHiHat,
            DrumVoice::Tom1,
            DrumVoice::Tom2,
            DrumVoice::Tom3,
            DrumVoice::Clap,
            DrumVoice::Ride,
            DrumVoice::Cymbal,
            DrumVoice::Snare606,
            DrumVoice::BassDrum808,
            DrumVoice::Perc1,
        ]
        .iter()
        {
            synth.set_voice_settings(*voice as usize, edge);
        }

        synth.trigger(b8_idx, 1.0);
        assert!(
            !contains_nan(&mut synth, b8_idx, 200),
            "B8 should not produce NaN after all edge-case settings"
        );
    }

    #[test]
    fn bassdrum808_stays_audible_when_settings_changed_mid_envelope() {
        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let b8_idx = DrumVoice::BassDrum808 as usize;

        // Trigger B8 and let it reach the middle of its decay
        synth.trigger(b8_idx, 1.0);
        let _ = sum_voice_output(&mut synth, b8_idx, 500); // ~11ms into decay

        // Now change Cymbal settings (should not affect B8)
        let mut cy = VoiceSettings::cymbal();
        cy.decay = 3.0;
        cy.volume = 0.2;
        synth.set_voice_settings(DrumVoice::Cymbal as usize, cy);

        // Continue processing B8 tail
        let tail = sum_voice_output(&mut synth, b8_idx, 500);
        assert!(
            tail > 0.0,
            "B8 tail should remain audible after Cymbal set_voice_settings: {}",
            tail
        );

        // Trigger B8 again after the tail
        synth.trigger(b8_idx, 1.0);
        let after = sum_voice_output(&mut synth, b8_idx, 100);
        assert!(
            after > 0.0,
            "B8 retrigger should still produce sound: {}",
            after
        );
    }

    #[test]
    fn modular_default_layout_initializes_slot_4_as_tom_not_open_hihat() {
        let mut synth = DrumSynthesizer::new();
        let layout = crate::track::TrackLayoutState::default_layout();

        synth.initialize_with_layout(44100.0, &layout);

        assert!(matches!(
            synth.voices[3].as_deref(),
            Some(DrumVoiceKind::Tom(_))
        ));
    }

    /// Test the specific attack_time=0 envelope corruption scenario.
    /// If attack_time is set to 0 while attack_remaining > 0, the envelope
    /// value becomes -inf. On the next trigger with normal attack_time,
    /// the envelope value becomes NaN and stays NaN forever.
    #[test]
    fn bassdrum808_attack_time_zero_during_attack_causes_nan() {
        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let b8_idx = DrumVoice::BassDrum808 as usize;

        // 1. Trigger B8 with normal attack
        synth.trigger(b8_idx, 1.0);
        let mut outputs = [[0.0f32; 2]; crate::track::MAX_TRACKS];
        // Process just a few samples (still in attack ramp)
        for _ in 0..5 {
            synth.process_voice_samples_stereo(&mut outputs);
        }
        assert!(
            outputs[b8_idx][0].is_finite(),
            "B8 should produce finite output during attack"
        );

        // 2. WHILE B8 is in attack phase, change its attack to 0.0
        // This simulates what would happen if set_voice_settings is called
        // with attack=0 while the envelope is attacking.
        let mut b8_zero_attack = VoiceSettings::kick808();
        b8_zero_attack.attack = 0.0;
        synth.set_voice_settings(DrumVoice::BassDrum808 as usize, b8_zero_attack);

        // Process a few more samples — the envelope should now be corrupted
        for _ in 0..10 {
            synth.process_voice_samples_stereo(&mut outputs);
        }

        // 3. Now set attack back to normal and trigger again
        let mut b8_normal = VoiceSettings::kick808();
        b8_normal.attack = 0.0015;
        synth.set_voice_settings(DrumVoice::BassDrum808 as usize, b8_normal);
        synth.trigger(b8_idx, 1.0);

        // 4. Check if B8 is permanently silent (NaN or 0)
        let mut has_finite_output = false;
        for _ in 0..100 {
            synth.process_voice_samples_stereo(&mut outputs);
            if outputs[b8_idx][0].is_finite() && outputs[b8_idx][0].abs() > 0.0001 {
                has_finite_output = true;
                break;
            }
        }
        assert!(
            has_finite_output,
            "B8 should recover and produce finite output after attack corruption"
        );
    }

    /// Regression test for the "plock click" bug.
    ///
    /// Scenario: lib.rs used to call `set_voice_settings` inside `iter_samples`,
    /// which meant a global settings update could overwrite a per-step plock
    /// in the middle of a buffer — producing a one-sample discontinuity.
    ///
    /// This test reproduces that exact situation:
    ///   1. Trigger kick at 60 Hz
    ///   2. Process 500 samples (tail develops)
    ///   3. `set_voice_settings` to 200 Hz (simulates the mid-buffer overwrite)
    ///   4. Process 200 more samples
    ///   5. Measure the spectral flux around the change point.
    ///
    /// With the old code (FREQ_SMOOTH_MS = 0.1 ms, no filter cutoff smoother)
    /// this produced a sharp HF spike (click).  With the fix it should be smooth.
    #[test]
    fn kick_no_click_when_settings_changed_mid_envelope() {
        use std::f32::consts::PI;

        let sample_rate = 44100.0;
        let mut synth = DrumSynthesizer::new();
        synth.initialize(sample_rate);

        let kick_idx = DrumVoice::Kick as usize;

        // Settings A: low frequency
        let mut settings_a = VoiceSettings::kick();
        settings_a.frequency = 60.0;
        settings_a.special[0] = 0.0; // disable click transient to isolate body
        settings_a.filter_freq = 2000.0;
        settings_a.filter_env_amount = 0.0;
        settings_a.decay = 0.3;
        settings_a.analog = 1.0;

        // Settings B: high frequency (the plock value)
        let mut settings_b = settings_a;
        settings_b.frequency = 200.0;

        // 1. Trigger at 60 Hz
        synth.set_voice_settings(DrumVoice::Kick as usize, settings_a);
        synth.trigger(kick_idx, 1.0);

        let mut outputs = [[0.0f32; 2]; crate::track::MAX_TRACKS];
        let mut samples: Vec<f32> = Vec::with_capacity(700);

        // 2. Process 500 samples
        for _ in 0..500 {
            synth.process_voice_samples_stereo(&mut outputs);
            samples.push(outputs[kick_idx][0]);
        }

        // 3. MID-ENVELOPE SETTINGS CHANGE — this is the bug scenario
        synth.set_voice_settings(DrumVoice::Kick as usize, settings_b);

        // 4. Process 200 more samples
        for _ in 0..200 {
            synth.process_voice_samples_stereo(&mut outputs);
            samples.push(outputs[kick_idx][0]);
        }

        // 5. Detect click: compute high-frequency energy in a 10 ms window
        //    centred on the change point (sample 500).
        let window = (sample_rate * 0.01) as usize; // 10 ms
        let start = 500usize.saturating_sub(window / 2);
        let end = (500 + window / 2).min(samples.len());

        // Simple 1-pole HP @ 3 kHz
        let mut hp_state = 0.0_f32;
        let alpha = 1.0 - (-2.0 * PI * 3000.0 / sample_rate).exp();
        let mut energy_low = 0.0_f32;
        let mut energy_high = 0.0_f32;
        for s in &samples[start..end] {
            energy_low += s * s;
            hp_state += alpha * (s - hp_state);
            let hp = s - hp_state;
            energy_high += hp * hp;
        }
        let hf_ratio = if energy_low > 0.0 {
            energy_high / energy_low
        } else {
            0.0
        };

        // Also compute maximum sample-to-sample delta in the window
        let mut max_delta = 0.0_f32;
        for i in start..end.saturating_sub(1) {
            max_delta = max_delta.max((samples[i + 1] - samples[i]).abs());
        }

        eprintln!("\n=== kick_no_click_when_settings_changed_mid_envelope ===");
        eprintln!("HF ratio around change: {}", hf_ratio);
        eprintln!("Max sample delta:       {}", max_delta);

        // With the fix the HF ratio should stay below 0.05 and delta below 0.05
        // (empirically: with FREQ_SMOOTH_MS=0.1ms, hf_ratio ~0.15 and delta ~0.25)
        assert!(
            hf_ratio < 0.08,
            "Click detected: HF energy spike at settings change. hf_ratio={}",
            hf_ratio
        );
        assert!(
            max_delta < 0.08,
            "Click detected: large sample delta at settings change. max_delta={}",
            max_delta
        );
    }

    /// Lightweight sanity check: every engine voice renders finite, non-silent audio
    /// with its default settings. This catches global regressions (silence, NaN, stereo
    /// collapse) after refactors like per-slot instances or special-param changes.
    #[test]
    fn all_voices_render_finite_non_silent_output() {
        const SAMPLES: usize = 200; // ~4.5 ms @ 44100 Hz — enough to catch silence/NaN
        const MIN_PEAK: f32 = 1e-4;

        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let default_settings: [(DrumVoice, fn() -> VoiceSettings); 13] = [
            (DrumVoice::Kick, VoiceSettings::kick),
            (DrumVoice::Snare, VoiceSettings::snare),
            (DrumVoice::HiHat, VoiceSettings::hihat),
            (DrumVoice::OpenHiHat, VoiceSettings::open_hihat),
            (DrumVoice::Tom1, VoiceSettings::tom1),
            (DrumVoice::Tom2, VoiceSettings::tom2),
            (DrumVoice::Tom3, VoiceSettings::tom3),
            (DrumVoice::Clap, VoiceSettings::clap),
            (DrumVoice::Ride, VoiceSettings::ride),
            (DrumVoice::Cymbal, VoiceSettings::cymbal),
            (DrumVoice::Snare606, VoiceSettings::snare606),
            (DrumVoice::BassDrum808, VoiceSettings::kick808),
            (DrumVoice::Perc1, VoiceSettings::perc1),
        ];

        let mut outputs = [[0.0f32; 2]; crate::track::MAX_TRACKS];
        for (voice, settings_fn) in default_settings.iter() {
            let voice_idx = *voice as usize;
            synth.set_voice_settings(voice_idx, settings_fn());
            synth.trigger(voice_idx, 1.0);

            let mut peak = 0.0f32;
            for _ in 0..SAMPLES {
                synth.process_voice_samples_stereo(&mut outputs);
                let left = outputs[voice_idx][0];
                let right = outputs[voice_idx][1];
                assert!(
                    left.is_finite() && right.is_finite(),
                    "{:?} produced non-finite samples (NaN/Inf) with default settings",
                    voice
                );
                peak = peak.max(left.abs()).max(right.abs());
            }

            assert!(
                peak >= MIN_PEAK,
                "{:?} is silent with default settings (peak = {})",
                voice,
                peak
            );
        }
    }

    /// Regression guard: retriggering any voice twice in quick succession must stay
    /// finite. Some bugs only appear on the second trigger (phase/envelope state).
    #[test]
    fn all_voices_stay_finite_on_retrigger() {
        const SAMPLES: usize = 100;

        let mut synth = DrumSynthesizer::new();
        synth.initialize(44100.0);

        let voices = [
            DrumVoice::Kick,
            DrumVoice::Snare,
            DrumVoice::HiHat,
            DrumVoice::OpenHiHat,
            DrumVoice::Tom1,
            DrumVoice::Tom2,
            DrumVoice::Tom3,
            DrumVoice::Clap,
            DrumVoice::Ride,
            DrumVoice::Cymbal,
            DrumVoice::Snare606,
            DrumVoice::BassDrum808,
            DrumVoice::Perc1,
        ];

        let mut outputs = [[0.0f32; 2]; crate::track::MAX_TRACKS];
        for voice in voices.iter() {
            let voice_idx = *voice as usize;
            synth.trigger(voice_idx, 1.0);
            // First trigger
            for _ in 0..SAMPLES {
                synth.process_voice_samples_stereo(&mut outputs);
            }
            // Immediate retrigger while tail may be ringing
            synth.trigger(voice_idx, 1.0);
            for _ in 0..SAMPLES {
                synth.process_voice_samples_stereo(&mut outputs);
                let left = outputs[voice_idx][0];
                let right = outputs[voice_idx][1];
                assert!(
                    left.is_finite() && right.is_finite(),
                    "{:?} produced non-finite sample on retrigger",
                    voice
                );
            }
        }
    }
}

