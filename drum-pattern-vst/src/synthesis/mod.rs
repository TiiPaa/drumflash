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
pub use settings::kick::KickSettings;
pub use settings::snare::SnareSettings;
pub use settings::hihat::HiHatSettings;
pub use settings::open_hihat::OpenHiHatSettings;
pub use settings::tom::TomSettings;
pub use settings::clap::ClapSettings;
pub use settings::ride::RideSettings;
pub use settings::cymbal::CymbalSettings;
pub use settings::snare606::Snare606Settings;
pub use settings::kick_808::Kick808Settings;
pub use settings::perc1::Perc1Settings;
pub use kick_808::Kick808Voice;
pub use open_hihat::OpenHiHatVoice;
pub use perc1::Perc1Voice;
pub use ride::RideVoice;
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
    pub special: [f32; 8],
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
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.0; 8],
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
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.5, 0.01, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 1.0,
            stereo: 1.0,
            algo: 0,
            special: [0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 1.0,
            stereo: 1.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn tom1() -> Self {
        Self {
            frequency: 300.0,
            decay: 0.3,
            volume: 0.5,
            filter_freq: 500.0,
            attack: 0.0015,
            release: 0.3,
            decay_curve: 4.2,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.06,
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn tom2() -> Self {
        Self {
            frequency: 200.0,
            decay: 0.4,
            volume: 0.5,
            filter_freq: 500.0,
            attack: 0.0015,
            release: 0.4,
            decay_curve: 4.2,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.06,
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn tom3() -> Self {
        Self {
            frequency: 120.0,
            decay: 0.5,
            volume: 0.5,
            filter_freq: 500.0,
            attack: 0.0015,
            release: 0.5,
            decay_curve: 4.2,
            release_curve: 3.0,
            hold: 0.0,
            filter_env_amount: 1.0,
            filter_env_decay: 0.06,
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 1.0,
            stereo: 1.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 1.0,
            stereo: 1.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 1.0,
            stereo: 1.0,
            algo: 0,
            special: [0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            // special[0] = Resonance (Q)
            // special[1] = Tone (body vs wires balance)
            // special[2] = Snap (crispness of wires layer)
            special: [4.5, 0.55, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 1.0,
            stereo: 0.0,
            algo: 0,
            // special[0] = Accent (click level)
            // special[1] = Snap (pitch sweep depth)
            // special[2] = Pitch Drop amount
            // Defaults at 0 so the user hears the difference when raising sliders.
            special: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
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
            analog: 0.3,
            stereo: 1.0,
            algo: 0,
            // special[0] = Sweep amount (-1..1)
            // special[1] = Sweep speed (ms)
            // special[2] = Bite (FM amount)
            // special[3] = Width (stereo + slap delay)
            special: [0.5, 80.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }
}

pub trait Voice: Send + Sync {
    fn trigger(&mut self);
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

pub struct DrumSynthesizer {
    voices: Vec<DrumVoiceKind>,
    sample_rate: f32,
    velocities: [f32; DrumVoice::COUNT],
    /// One-pole smoothers on each voice's velocity, absorbing gain jumps when
    /// retriggering a voice while its tail is still ringing.
    velocity_smoothers: [dsp::OnePoleSmoother; DrumVoice::COUNT],
}

const VELOCITY_SMOOTH_MS: f32 = 1.5;

impl DrumSynthesizer {
    pub fn new() -> Self {
        Self {
            voices: Vec::with_capacity(DrumVoice::COUNT),
            sample_rate: 44100.0,
            velocities: [1.0; DrumVoice::COUNT],
            velocity_smoothers: std::array::from_fn(|_| {
                dsp::OnePoleSmoother::new(44100.0, VELOCITY_SMOOTH_MS, 1.0)
            }),
        }
    }

    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.voices.clear();
        for smoother in self.velocity_smoothers.iter_mut() {
            *smoother = dsp::OnePoleSmoother::new(sample_rate, VELOCITY_SMOOTH_MS, 1.0);
        }

        // Create all 13 voices with dedicated models.
        self.voices.push(DrumVoiceKind::Kick(KickVoice::new(
            sample_rate,
            KickSettings::from(VoiceSettings::kick()),
        )));
        self.voices.push(DrumVoiceKind::Snare(SnareVoice::new(
            sample_rate,
            SnareSettings::from(VoiceSettings::snare()),
        )));
        self.voices.push(DrumVoiceKind::HiHat(HiHatVoice::new(
            sample_rate,
            HiHatSettings::from(VoiceSettings::hihat()),
        )));
        self.voices
            .push(DrumVoiceKind::OpenHiHat(OpenHiHatVoice::new(
                sample_rate,
                OpenHiHatSettings::from(VoiceSettings::open_hihat()),
            )));
        self.voices.push(DrumVoiceKind::Tom(TomVoice::new(
            sample_rate,
            TomSettings::from(VoiceSettings::tom1()),
        )));
        self.voices.push(DrumVoiceKind::Tom(TomVoice::new(
            sample_rate,
            TomSettings::from(VoiceSettings::tom2()),
        )));
        self.voices.push(DrumVoiceKind::Tom(TomVoice::new(
            sample_rate,
            TomSettings::from(VoiceSettings::tom3()),
        )));
        self.voices.push(DrumVoiceKind::Clap(ClapVoice::new(
            sample_rate,
            ClapSettings::from(VoiceSettings::clap()),
        )));
        self.voices.push(DrumVoiceKind::Ride(RideVoice::new(
            sample_rate,
            RideSettings::from(VoiceSettings::ride()),
        )));
        self.voices.push(DrumVoiceKind::Cymbal(CymbalVoice::new(
            sample_rate,
            CymbalSettings::from(VoiceSettings::cymbal()),
        )));
        self.voices.push(DrumVoiceKind::Snare606(Snare606Voice::new(
            sample_rate,
            Snare606Settings::from(VoiceSettings::snare606()),
        )));
        self.voices
            .push(DrumVoiceKind::BassDrum808(Kick808Voice::new(
                sample_rate,
                Kick808Settings::from(VoiceSettings::kick808()),
            )));
        self.voices.push(DrumVoiceKind::Perc1(Perc1Voice::new(
            sample_rate,
            Perc1Settings::from(VoiceSettings::perc1()),
        )));
    }

    pub fn trigger(&mut self, voice_idx: usize, velocity: f32) {
        if let Some(voice) = self.voices.get_mut(voice_idx) {
            voice.trigger();
            self.velocities[voice_idx] = velocity;
        }
    }

    #[allow(dead_code)]
    pub fn process_sample(&mut self, output: &mut f32) {
        let mut mixed = 0.0f32;
        for voice in &mut self.voices {
            mixed += voice.process_sample();
        }
        *output = mixed;
    }

    #[allow(dead_code)]
    pub fn process_voice_samples(&mut self, outputs: &mut [f32; DrumVoice::COUNT]) {
        for (i, (voice, output)) in self.voices.iter_mut().zip(outputs.iter_mut()).enumerate() {
            let vel = self.velocity_smoothers[i].process(self.velocities[i]);
            *output = voice.process_sample() * vel;
        }
    }

    pub fn process_voice_samples_stereo(&mut self, outputs: &mut [[f32; 2]; DrumVoice::COUNT]) {
        for (i, (voice, output)) in self.voices.iter_mut().zip(outputs.iter_mut()).enumerate() {
            let vel = self.velocity_smoothers[i].process(self.velocities[i]);
            let (l, r) = voice.process_sample_stereo();
            output[0] = l * vel;
            output[1] = r * vel;
        }
    }

    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.reset();
        }
    }

    #[allow(dead_code)]
    pub fn set_voice_settings(&mut self, voice: DrumVoice, settings: VoiceSettings) {
        if let Some(v) = self.voices.get_mut(voice as usize) {
            v.set_settings(settings);
        }
    }

    #[allow(dead_code)]
    pub fn set_special_param(&mut self, voice_idx: usize, index: usize, value: f32) {
        if let Some(v) = self.voices.get_mut(voice_idx) {
            v.set_special_param(index, value);
        }
    }

    pub fn set_algo(&mut self, voice_idx: usize, algo: u8) {
        if let Some(v) = self.voices.get_mut(voice_idx) {
            v.set_algo(algo);
        }
    }

    pub fn reset_voice(&mut self, voice_idx: usize) {
        if let Some(v) = self.voices.get_mut(voice_idx) {
            v.reset();
        }
    }
}
