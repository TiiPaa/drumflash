//! Factory presets embedded in the binary.
//!
//! Authoring workflow (debug builds): compose the sound/pattern/song in the
//! plugin, open the Presets modal and use the "Export factory" button — the
//! JSON lands in `Documents/Flash Drum/presets/_factory/<kind>/`. Then copy
//! the file into `assets/presets/<kind>/`, add one `include_str!` line below,
//! and commit.
//!
//! Each entry is the raw JSON of the corresponding preset type
//! (`presets::InstrumentPreset` / `PatternPreset` / `SongPreset`).

/// Factory instrument presets (`assets/presets/instruments/*.fdinst.json`).
pub const INSTRUMENTS: &[&str] = &[
    // include_str!("../assets/presets/instruments/example.fdinst.json"),
];

/// Factory pattern presets (`assets/presets/patterns/*.fdpat.json`).
pub const PATTERNS: &[&str] = &[
    // include_str!("../assets/presets/patterns/example.fdpat.json"),
];

/// Factory grid (lane kit) presets (`assets/presets/grids/*.fdgrid.json`).
/// The built-in layouts (Clear All / 4 Lanes / 12 Lanes) are code, not files.
pub const GRIDS: &[&str] = &[
    // include_str!("../assets/presets/grids/example.fdgrid.json"),
];

/// Factory song presets (`assets/presets/songs/*.fdsong.json`).
pub const SONGS: &[&str] = &[
    // include_str!("../assets/presets/songs/example.fdsong.json"),
];
