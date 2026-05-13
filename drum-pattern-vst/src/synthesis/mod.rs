//! Audio synthesis module for drum sounds

mod dsp;
mod special_params;
mod clap;
mod cymbal;
mod hihat;
mod kick;
mod open_hihat;
mod ride;
mod snare;
mod tom;

pub use special_params::{AlgoDef, SpecialParamDef, algos_for, specials_for};

pub use clap::ClapVoice;
pub use cymbal::CymbalVoice;
pub use hihat::HiHatVoice;
pub use kick::KickVoice;
pub use open_hihat::OpenHiHatVoice;
pub use ride::RideVoice;
pub use snare::SnareVoice;
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
}

#[allow(dead_code)]
impl DrumVoice {
    pub const COUNT: usize = 10;

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
            _ => None,
        }
    }

    pub fn midi_note(&self) -> u8 {
        match self {
            DrumVoice::Kick => 36,
            DrumVoice::Snare => 38,
            DrumVoice::HiHat => 42,
            DrumVoice::OpenHiHat => 46,
            DrumVoice::Tom1 => 50,
            DrumVoice::Tom2 => 47,
            DrumVoice::Tom3 => 43,
            DrumVoice::Clap => 39,
            DrumVoice::Ride => 51,
            DrumVoice::Cymbal => 49,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DrumVoice::Kick => "Kick",
            DrumVoice::Snare => "Snare",
            DrumVoice::HiHat => "Hi-Hat",
            DrumVoice::OpenHiHat => "Open HH",
            DrumVoice::Tom1 => "Tom 1",
            DrumVoice::Tom2 => "Tom 2",
            DrumVoice::Tom3 => "Tom 3",
            DrumVoice::Clap => "Clap",
            DrumVoice::Ride => "Ride",
            DrumVoice::Cymbal => "Cymbal",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceSettings {
    pub frequency: f32,
    pub decay: f32,
    pub volume: f32,
    pub filter_freq: f32,
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
            filter_freq: 100.0,
            algo: 0,
            special: [0.5, 0.01, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn snare() -> Self {
        Self {
            frequency: 200.0,
            decay: 0.47,
            volume: 0.6,
            filter_freq: 1000.0,
            algo: 0,
            special: [0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn hihat() -> Self {
        Self {
            frequency: 8000.0,
            decay: 0.36,
            volume: 0.3,
            filter_freq: 10000.0,
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
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn tom1() -> Self {
        Self {
            frequency: 300.0,
            decay: 0.3,
            volume: 0.5,
            filter_freq: 2000.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn tom2() -> Self {
        Self {
            frequency: 200.0,
            decay: 0.4,
            volume: 0.5,
            filter_freq: 1500.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn tom3() -> Self {
        Self {
            frequency: 120.0,
            decay: 0.5,
            volume: 0.5,
            filter_freq: 1000.0,
            algo: 0,
            special: [0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }

    pub fn clap() -> Self {
        Self {
            frequency: 1200.0,
            decay: 0.15,
            volume: 0.7,
            filter_freq: 2500.0,
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
            algo: 0,
            special: [0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        }
    }
}

pub trait Voice: Send + Sync {
    fn trigger(&mut self);
    fn process_sample(&mut self) -> f32;
    #[allow(dead_code)]
    fn is_active(&self) -> bool;
    fn reset(&mut self);
    #[allow(dead_code)]
    fn set_settings(&mut self, settings: VoiceSettings);

    /// Set synthesis algorithm by index.
    fn set_algo(&mut self, algo: u8);
    /// Set a special parameter by index (0..7).
    fn set_special_param(&mut self, index: usize, value: f32);
    /// Supported algorithms for this voice.
    fn supported_algos(&self) -> &'static [AlgoDef];
    /// Supported special parameters for this voice.
    fn special_params(&self) -> &'static [SpecialParamDef];
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
        }
    }

    fn supported_algos(&self) -> &'static [AlgoDef] {
        match self {
            DrumVoiceKind::Kick(v) => v.supported_algos(),
            DrumVoiceKind::Snare(v) => v.supported_algos(),
            DrumVoiceKind::HiHat(v) => v.supported_algos(),
            DrumVoiceKind::OpenHiHat(v) => v.supported_algos(),
            DrumVoiceKind::Tom(v) => v.supported_algos(),
            DrumVoiceKind::Clap(v) => v.supported_algos(),
            DrumVoiceKind::Ride(v) => v.supported_algos(),
            DrumVoiceKind::Cymbal(v) => v.supported_algos(),
        }
    }

    fn special_params(&self) -> &'static [SpecialParamDef] {
        match self {
            DrumVoiceKind::Kick(v) => v.special_params(),
            DrumVoiceKind::Snare(v) => v.special_params(),
            DrumVoiceKind::HiHat(v) => v.special_params(),
            DrumVoiceKind::OpenHiHat(v) => v.special_params(),
            DrumVoiceKind::Tom(v) => v.special_params(),
            DrumVoiceKind::Clap(v) => v.special_params(),
            DrumVoiceKind::Ride(v) => v.special_params(),
            DrumVoiceKind::Cymbal(v) => v.special_params(),
        }
    }
}

pub struct DrumSynthesizer {
    voices: Vec<DrumVoiceKind>,
    sample_rate: f32,
    velocities: [f32; DrumVoice::COUNT],
}

impl DrumSynthesizer {
    pub fn new() -> Self {
        Self {
            voices: Vec::with_capacity(DrumVoice::COUNT),
            sample_rate: 44100.0,
            velocities: [1.0; DrumVoice::COUNT],
        }
    }

    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.voices.clear();

        // Create all 10 voices with dedicated models.
        self.voices.push(DrumVoiceKind::Kick(KickVoice::new(
            sample_rate,
            VoiceSettings::kick(),
        )));
        self.voices.push(DrumVoiceKind::Snare(SnareVoice::new(
            sample_rate,
            VoiceSettings::snare(),
        )));
        self.voices.push(DrumVoiceKind::HiHat(HiHatVoice::new(
            sample_rate,
            VoiceSettings::hihat(),
        )));
        self.voices.push(DrumVoiceKind::OpenHiHat(OpenHiHatVoice::new(
            sample_rate,
            VoiceSettings::open_hihat(),
        )));
        self.voices.push(DrumVoiceKind::Tom(TomVoice::new(
            sample_rate,
            VoiceSettings::tom1(),
        )));
        self.voices.push(DrumVoiceKind::Tom(TomVoice::new(
            sample_rate,
            VoiceSettings::tom2(),
        )));
        self.voices.push(DrumVoiceKind::Tom(TomVoice::new(
            sample_rate,
            VoiceSettings::tom3(),
        )));
        self.voices.push(DrumVoiceKind::Clap(ClapVoice::new(
            sample_rate,
            VoiceSettings::clap(),
        )));
        self.voices.push(DrumVoiceKind::Ride(RideVoice::new(
            sample_rate,
            VoiceSettings::ride(),
        )));
        self.voices.push(DrumVoiceKind::Cymbal(CymbalVoice::new(
            sample_rate,
            VoiceSettings::cymbal(),
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

    pub fn process_voice_samples(&mut self, outputs: &mut [f32; DrumVoice::COUNT]) {
        for (i, (voice, output)) in self.voices.iter_mut().zip(outputs.iter_mut()).enumerate() {
            *output = voice.process_sample() * self.velocities[i];
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
