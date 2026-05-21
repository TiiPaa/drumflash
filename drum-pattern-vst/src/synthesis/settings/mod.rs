//! Typed per-instrument settings — replaces opaque `special: [f32; 8]`.
//!
//! Each instrument gets its own struct with named fields for standard
//! and special parameters.  `VoiceSettings` remains the serialization/
//! persistence format; conversion happens inside each voice's
//! `set_settings()` wrapper.

pub mod kick;
