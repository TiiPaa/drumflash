// ============================================================
// Engine Registry — moteurs assignables par lane
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    Synth,
    Sample,
    MidiOut,
}

impl Engine {
    pub fn label(&self) -> &'static str {
        match self {
            Engine::Synth => "Synth",
            Engine::Sample => "Sample",
            Engine::MidiOut => "MIDI Out",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Engine::Synth => "⚡",
            Engine::Sample => "🎵",
            Engine::MidiOut => "🔌",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineGroup {
    Oscillator,
    Noise,
    Filter,
    Envelope,
    Velocity,
    FX,
    Midi,
}

impl EngineGroup {
    pub fn label(&self) -> &'static str {
        match self {
            EngineGroup::Oscillator => "OSC",
            EngineGroup::Noise => "NOISE",
            EngineGroup::Filter => "FILTER",
            EngineGroup::Envelope => "ENVELOPE",
            EngineGroup::Velocity => "VELOCITY",
            EngineGroup::FX => "FX",
            EngineGroup::Midi => "MIDI",
        }
    }
}

/// Retourne les groupes de paramètres pour un moteur donné.
pub fn schema_for_engine(engine: Engine) -> Vec<EngineGroup> {
    match engine {
        Engine::Synth => vec![
            EngineGroup::Oscillator,
            EngineGroup::Noise,
            EngineGroup::Filter,
            EngineGroup::Envelope,
            EngineGroup::Velocity,
            EngineGroup::FX,
        ],
        Engine::Sample => vec![
            EngineGroup::Envelope,
            EngineGroup::Velocity,
            EngineGroup::FX,
        ],
        Engine::MidiOut => vec![EngineGroup::Midi],
    }
}

/// Retourne tous les moteurs disponibles.
pub fn available_engines() -> Vec<Engine> {
    vec![Engine::Synth, Engine::Sample, Engine::MidiOut]
}
