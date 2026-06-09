//! Style definitions for musical pattern generation.
//! Each style provides *structured* rhythmic roles rather than raw probabilities.

use crate::sequencer::pattern::{Pattern, INSTRUMENT_COUNT, STEP_COUNT};
use nih_plug::prelude::*;

/// Available musical styles.
#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Style {
    #[id = "rock"]
    #[name = "Rock"]
    #[default]
    Rock,
    #[id = "funk"]
    #[name = "Funk"]
    Funk,
    #[id = "techno"]
    #[name = "Techno"]
    Techno,
    #[id = "hiphop"]
    #[name = "Hip-Hop"]
    HipHop,
    #[id = "jazz"]
    #[name = "Jazz"]
    Jazz,
    #[id = "metal"]
    #[name = "Metal"]
    Metal,
    #[id = "latin"]
    #[name = "Latin"]
    Latin,
    #[id = "disco"]
    #[name = "Disco"]
    Disco,
    #[id = "trap"]
    #[name = "Trap"]
    Trap,
    #[id = "reggae"]
    #[name = "Reggae"]
    Reggae,
}

impl Style {
    pub fn label(&self) -> &'static str {
        match self {
            Style::Rock => "Rock",
            Style::Funk => "Funk",
            Style::Techno => "Techno",
            Style::HipHop => "Hip-Hop",
            Style::Jazz => "Jazz",
            Style::Metal => "Metal",
            Style::Latin => "Latin",
            Style::Disco => "Disco",
            Style::Trap => "Trap",
            Style::Reggae => "Reggae",
        }
    }
}

/// Rhythmic role for an instrument within a style.
/// Describes *where* an instrument tends to play, not just probabilities.
#[derive(Clone, Debug)]
pub struct RhythmicRole {
    /// Steps where the instrument MUST play (foundational hits).
    pub anchors: &'static [usize],
    /// Steps where the instrument MAY play (subdivisions, ghost notes).
    pub candidates: &'static [usize],
    /// Probability of playing at a candidate step (0.0..1.0).
    pub candidate_prob: f32,
    /// Steps where the instrument should NEVER play (exclusion zones).
    pub exclusions: &'static [usize],
}

/// A musical template defines the structured roles for all instruments.
pub struct MusicalTemplate {
    pub roles: [RhythmicRole; INSTRUMENT_COUNT],
    /// Typical BPM range for this style.
    pub bpm_range: (f32, f32),
}

impl MusicalTemplate {
    pub fn for_style(style: Style) -> Self {
        match style {
            Style::Rock => Self::rock(),
            Style::Funk => Self::funk(),
            Style::Techno => Self::techno(),
            Style::HipHop => Self::hiphop(),
            Style::Jazz => Self::jazz(),
            Style::Metal => Self::metal(),
            Style::Latin => Self::latin(),
            Style::Disco => Self::disco(),
            Style::Trap => Self::trap(),
            Style::Reggae => Self::reggae(),
        }
    }

    fn rock() -> Self {
        Self {
            roles: [
                // Kick: on 1,3 + optional syncopation
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Snare: backbeat on 2,4
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.1,
                    exclusions: &[0, 8],
                },
                // HiHat: steady 8ths
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Open HH: offbeats occasionally
                RhythmicRole {
                    anchors: &[],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.25,
                    exclusions: &[0, 4, 8, 12],
                },
                // Toms: fill territory only
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.2,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Snare 606: tight backbeat layer, lighter than main snare
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 6, 10],
                    candidate_prob: 0.35,
                    exclusions: &[0, 8],
                },
                // 808 Kick: sub-bass reinforcement on downbeats only
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // Perc1: crash/FX accents and downbeat emphasis
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15, 7, 11],
                    candidate_prob: 0.2,
                    exclusions: &[],
                },
            ],
            bpm_range: (90.0, 140.0),
        }
    }

    fn funk() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[3, 6, 10, 12],
                    candidate_prob: 0.5,
                    exclusions: &[1, 2, 4, 5, 7, 9, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[7, 14],
                    candidate_prob: 0.35,
                    exclusions: &[0, 8],
                },
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.4,
                    exclusions: &[0, 4, 8, 12],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.25,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.2,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (90.0, 130.0),
        }
    }

    fn techno() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[0, 4, 8, 12],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.1,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[0, 8],
                    candidate_prob: 0.05,
                    exclusions: &[1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.3,
                    exclusions: &[0, 4, 8, 12],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (120.0, 140.0),
        }
    }

    fn hiphop() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[6, 10, 12, 14],
                    candidate_prob: 0.35,
                    exclusions: &[1, 2, 3, 4, 5, 7, 9, 11, 13, 15],
                },
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[0, 8],
                },
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[3, 7, 11, 15],
                    candidate_prob: 0.2,
                    exclusions: &[0, 4, 8, 12],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (80.0, 100.0),
        }
    }

    fn jazz() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12],
                    candidate_prob: 0.2,
                    exclusions: &[1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.1,
                    exclusions: &[0, 8],
                },
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[0, 4, 8, 12],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.25,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.2,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (100.0, 160.0),
        }
    }

    fn metal() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.4,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[0, 8],
                    candidate_prob: 0.25,
                    exclusions: &[1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.2,
                    exclusions: &[0, 4, 8, 12],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.3,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.25,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.2,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.2,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (140.0, 200.0),
        }
    }

    fn latin() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[3, 6, 10, 12],
                    candidate_prob: 0.4,
                    exclusions: &[1, 2, 4, 5, 7, 9, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[7, 15],
                    candidate_prob: 0.3,
                    exclusions: &[0, 8],
                },
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.25,
                    exclusions: &[0, 4, 8, 12],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.25,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.2,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (100.0, 140.0),
        }
    }

    fn disco() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[0, 4, 8, 12],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.1,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[0, 8],
                    candidate_prob: 0.05,
                    exclusions: &[1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[3, 7, 11, 15],
                    candidates: &[1, 5, 9, 13],
                    candidate_prob: 0.15,
                    exclusions: &[0, 2, 4, 6, 8, 10, 12, 14],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.2,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (110.0, 130.0),
        }
    }

    fn trap() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[7, 10, 12],
                    candidate_prob: 0.5,
                    exclusions: &[1, 2, 3, 4, 5, 6, 9, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[8],
                    candidates: &[4, 12],
                    candidate_prob: 0.3,
                    exclusions: &[0, 1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.4,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12],
                    candidate_prob: 0.25,
                    exclusions: &[0, 8],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (130.0, 160.0),
        }
    }

    fn reggae() -> Self {
        Self {
            roles: [
                RhythmicRole {
                    anchors: &[8],
                    candidates: &[0, 4, 12],
                    candidate_prob: 0.2,
                    exclusions: &[1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[4, 12],
                    candidates: &[0, 8],
                    candidate_prob: 0.1,
                    exclusions: &[1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14, 15],
                },
                RhythmicRole {
                    anchors: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidates: &[1, 3, 5, 7, 9, 11, 13, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[2, 6, 10, 14],
                    candidate_prob: 0.2,
                    exclusions: &[0, 4, 8, 12],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
                },
                // Clap
                RhythmicRole {
                    anchors: &[],
                    candidates: &[4, 12, 14, 15],
                    candidate_prob: 0.1,
                    exclusions: &[],
                },
                // Ride
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 2, 4, 6, 8, 10, 12, 14],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Cymbal
                RhythmicRole {
                    anchors: &[],
                    candidates: &[0, 14, 15],
                    candidate_prob: 0.05,
                    exclusions: &[],
                },
                // Snare 606 (no auto-generation — user-only)
                RhythmicRole {
                    anchors: &[],
                    candidates: &[],
                    candidate_prob: 0.0,
                    exclusions: &[],
                },
                // 808 Kick: plays like a secondary kick
                RhythmicRole {
                    anchors: &[0, 8],
                    candidates: &[4, 12, 2, 6, 10, 14],
                    candidate_prob: 0.15,
                    exclusions: &[1, 3, 5, 7, 9, 11, 13, 15],
                },
                // Perc1: occasional fill / effect
                RhythmicRole {
                    anchors: &[],
                    candidates: &[14, 15],
                    candidate_prob: 0.15,
                    exclusions: &[],
                },
            ],
            bpm_range: (70.0, 90.0),
        }
    }
}

/// Generate a pattern from a musical template with density control.
/// Uses a two-pass approach: anchors first, then candidates with musical rules.
pub fn generate_from_template(
    template: &MusicalTemplate,
    density: f32,
    rng: &mut impl FnMut() -> f32,
) -> Pattern {
    let mut pattern = Pattern::empty();
    pattern.name = "Generated".to_string();
    let d = density.clamp(0.0, 1.0);

    for inst in 0..INSTRUMENT_COUNT {
        let role = &template.roles[inst];

        // Pass 1: anchors are always placed (foundational hits)
        for &step in role.anchors {
            pattern.steps[step].instruments[inst] = true;
        }

        // Pass 2: candidates with probability scaled by density
        for &step in role.candidates {
            let prob = role.candidate_prob * (0.3 + 0.7 * d);
            if rng() < prob {
                pattern.steps[step].instruments[inst] = true;
            }
        }
    }

    // Musical coherence pass: enforce exclusion zones
    for inst in 0..INSTRUMENT_COUNT {
        let role = &template.roles[inst];
        for &step in role.exclusions {
            pattern.steps[step].instruments[inst] = false;
        }
    }

    // Rule: avoid kick and snare on the same step unless density is very high (intentional chaos)
    if d < 0.85 {
        for step in 0..STEP_COUNT {
            if pattern.steps[step].instruments[0] && pattern.steps[step].instruments[1] {
                // Prefer the stronger instrument (snare on backbeat, kick elsewhere)
                if step == 4 || step == 12 {
                    pattern.steps[step].instruments[0] = false;
                } else {
                    pattern.steps[step].instruments[1] = false;
                }
            }
        }
    }

    // Rule: closed HH and open HH should not clash
    for step in 0..STEP_COUNT {
        if pattern.steps[step].instruments[2] && pattern.steps[step].instruments[3] {
            pattern.steps[step].instruments[2] = false; // Open HH wins
        }
    }

    // Density-based ghost note suppression for kick at low density
    if d < 0.4 {
        for step in 0..STEP_COUNT {
            if pattern.steps[step].instruments[0]
                && !(step == 0 || step == 8 || step == 4 || step == 12)
            {
                if rng() < 0.5 {
                    pattern.steps[step].instruments[0] = false;
                }
            }
        }
    }

    pattern
}

/// Interpolate between two templates.
pub fn mix_templates(a: &MusicalTemplate, b: &MusicalTemplate, t: f32) -> MusicalTemplate {
    let mix = t.clamp(0.0, 1.0);
    let mut roles = a.roles.clone();
    for inst in 0..INSTRUMENT_COUNT {
        // Interpolate candidate probabilities
        roles[inst].candidate_prob =
            a.roles[inst].candidate_prob * (1.0 - mix) + b.roles[inst].candidate_prob * mix;
    }
    MusicalTemplate {
        roles,
        bpm_range: (
            a.bpm_range.0 * (1.0 - mix) + b.bpm_range.0 * mix,
            a.bpm_range.1 * (1.0 - mix) + b.bpm_range.1 * mix,
        ),
    }
}
