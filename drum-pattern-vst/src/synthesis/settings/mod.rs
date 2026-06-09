//! Typed per-instrument settings — replaces opaque `special: [f32; 32]`.
//!
//! Each instrument gets its own struct with named fields for standard
//! and special parameters.  `VoiceSettings` remains the serialization/
//! persistence format; conversion happens inside each voice's
//! `set_settings()` wrapper.

pub mod clap;
pub mod cymbal;
pub mod hihat;
pub mod kick;
pub mod kick_808;
pub mod open_hihat;
pub mod perc1;
pub mod ride;
pub mod snare;
pub mod snare606;
pub mod tom;
