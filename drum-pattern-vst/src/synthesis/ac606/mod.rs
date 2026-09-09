//! 606-inspired synthesis voices ported from analogcode's
//! "606-Inspired-Synth-Drums" (<https://github.com/analogcode/606-Inspired-Synth-Drums>).
//!
//! MIT License — Copyright (c) 2026 Matthew Fecher.
//! The full license text ships next to this module (`LICENSE-MIT.txt`) as
//! required by the MIT terms, and the author is credited in the plugin's
//! About section. Many thanks to Matthew Fecher (analogcode / AudioKit Pro)
//! for open-sourcing these measurement-fitted drum voices.
//!
//! The ports are as faithful as possible: fitted constants, RNG streams and
//! filter topologies are unchanged so the rendered sound matches the
//! reference C++ implementation. Each engine is mono, allocation-free in
//! `process()`, and exposes `trigger(...) -> process() -> f32`.
//!
//! Integration: each engine is wrapped as an `*(AC)` algo of an existing
//! Flash Drum voice (see `special_params.rs`), which maps our standard
//! params (Frequency / Decay / Volume / Analog) onto the engine's knobs and
//! routes the output through the shared saturation chain + `RetrigDeclick`.

mod bass_drum;
mod clap;
mod common;
mod hats;
mod snare;
#[cfg(test)]
mod tests;
mod toms;

pub use bass_drum::{AcBassDrum, BdMods};
pub use clap::AcClap;
pub use hats::{AcMetalHat, HatMods, CLOSED_HAT_SPEC, OPEN_HAT_SPEC};
pub use snare::{AcSnare, SnareMods};
pub use toms::{AcTom, TomMods, HIGH_TOM_SPEC, LOW_TOM_SPEC};

/// Pre-build the clap's reconstruction table outside the audio thread.
/// Called once from `DrumSynthesizer::initialize_with_layout()`; the table is
/// behind a `OnceLock` so a cold call on the audio thread would still be
/// correct, just not real-time friendly.
pub fn prewarm() {
    clap::prewarm();
}
