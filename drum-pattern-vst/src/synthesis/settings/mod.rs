//! Typed per-instrument settings — replaces opaque `special: [f32; 8]`.
//!
//! Each instrument gets its own struct with named fields for standard
//! and special parameters.  `VoiceSettings` remains the serialization/
//! persistence format; conversion happens inside each voice's
//! `set_settings()` wrapper.

pub mod kick;
pub mod snare;
pub mod hihat;
pub mod open_hihat;
pub mod tom;
pub mod clap;
pub mod ride;
pub mod cymbal;
pub mod snare606;
pub mod kick_808;
pub mod perc1;
