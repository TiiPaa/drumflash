//! Audio synthesis module for drum sounds

mod hihat;
mod kick;
mod open_hihat;
mod snare;
mod tom;

pub use hihat::HiHatVoice;
pub use kick::KickVoice;
pub use open_hihat::OpenHiHatVoice;
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
}

#[allow(dead_code)]
impl DrumVoice {
    pub const COUNT: usize = 7;

    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Kick),
            1 => Some(Self::Snare),
            2 => Some(Self::HiHat),
            3 => Some(Self::OpenHiHat),
            4 => Some(Self::Tom1),
            5 => Some(Self::Tom2),
            6 => Some(Self::Tom3),
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
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoiceSettings {
    pub frequency: f32,
    pub decay: f32,
    pub volume: f32,
    pub filter_freq: f32,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            frequency: 60.0,
            decay: 0.5,
            volume: 0.8,
            filter_freq: 100.0,
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
        }
    }

    pub fn snare() -> Self {
        Self {
            frequency: 200.0,
            decay: 0.2,
            volume: 0.6,
            filter_freq: 1000.0,
        }
    }

    pub fn hihat() -> Self {
        Self {
            frequency: 8000.0,
            decay: 0.1,
            volume: 0.3,
            filter_freq: 10000.0,
        }
    }

    pub fn open_hihat() -> Self {
        Self {
            frequency: 6000.0,
            decay: 0.3,
            volume: 0.4,
            filter_freq: 8000.0,
        }
    }

    pub fn tom1() -> Self {
        Self {
            frequency: 300.0,
            decay: 0.3,
            volume: 0.5,
            filter_freq: 2000.0,
        }
    }

    pub fn tom2() -> Self {
        Self {
            frequency: 200.0,
            decay: 0.4,
            volume: 0.5,
            filter_freq: 1500.0,
        }
    }

    pub fn tom3() -> Self {
        Self {
            frequency: 120.0,
            decay: 0.5,
            volume: 0.5,
            filter_freq: 1000.0,
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
}

/// Concrete enum wrapping all drum voice types.
/// Eliminates dynamic dispatch from the audio path.
pub enum DrumVoiceKind {
    Kick(KickVoice),
    Snare(SnareVoice),
    HiHat(HiHatVoice),
    OpenHiHat(OpenHiHatVoice),
    Tom(TomVoice),
}

impl Voice for DrumVoiceKind {
    fn trigger(&mut self) {
        match self {
            DrumVoiceKind::Kick(v) => v.trigger(),
            DrumVoiceKind::Snare(v) => v.trigger(),
            DrumVoiceKind::HiHat(v) => v.trigger(),
            DrumVoiceKind::OpenHiHat(v) => v.trigger(),
            DrumVoiceKind::Tom(v) => v.trigger(),
        }
    }

    fn process_sample(&mut self) -> f32 {
        match self {
            DrumVoiceKind::Kick(v) => v.process_sample(),
            DrumVoiceKind::Snare(v) => v.process_sample(),
            DrumVoiceKind::HiHat(v) => v.process_sample(),
            DrumVoiceKind::OpenHiHat(v) => v.process_sample(),
            DrumVoiceKind::Tom(v) => v.process_sample(),
        }
    }

    fn is_active(&self) -> bool {
        match self {
            DrumVoiceKind::Kick(v) => v.is_active(),
            DrumVoiceKind::Snare(v) => v.is_active(),
            DrumVoiceKind::HiHat(v) => v.is_active(),
            DrumVoiceKind::OpenHiHat(v) => v.is_active(),
            DrumVoiceKind::Tom(v) => v.is_active(),
        }
    }

    fn reset(&mut self) {
        match self {
            DrumVoiceKind::Kick(v) => v.reset(),
            DrumVoiceKind::Snare(v) => v.reset(),
            DrumVoiceKind::HiHat(v) => v.reset(),
            DrumVoiceKind::OpenHiHat(v) => v.reset(),
            DrumVoiceKind::Tom(v) => v.reset(),
        }
    }

    fn set_settings(&mut self, settings: VoiceSettings) {
        match self {
            DrumVoiceKind::Kick(v) => v.set_settings(settings),
            DrumVoiceKind::Snare(v) => v.set_settings(settings),
            DrumVoiceKind::HiHat(v) => v.set_settings(settings),
            DrumVoiceKind::OpenHiHat(v) => v.set_settings(settings),
            DrumVoiceKind::Tom(v) => v.set_settings(settings),
        }
    }
}

pub struct DrumSynthesizer {
    voices: Vec<DrumVoiceKind>,
    sample_rate: f32,
}

impl DrumSynthesizer {
    pub fn new() -> Self {
        Self {
            voices: Vec::with_capacity(DrumVoice::COUNT),
            sample_rate: 44100.0,
        }
    }

    pub fn initialize(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.voices.clear();

        // Create all 7 voices with dedicated models.
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
    }

    pub fn trigger(&mut self, voice_idx: usize) {
        if let Some(voice) = self.voices.get_mut(voice_idx) {
            voice.trigger();
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
        for (voice, output) in self.voices.iter_mut().zip(outputs.iter_mut()) {
            *output = voice.process_sample();
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
}
