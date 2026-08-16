//! User & factory presets: instrument sounds, full patterns, songs.
//!
//! User presets live as versioned JSON files under
//! `Documents/Flash Drum/presets/{instruments,patterns,songs}/`.
//! Factory presets are embedded in the binary (see `factory_presets.rs`) and
//! are read-only in the UI.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::pattern_bank::{PatternSlot, SongSequence};
use crate::sequencer::SharedPattern;
use crate::sound_settings::{InstrumentSettingsState, SoundSettingsState};
use crate::track::{TrackInstrumentKind, MAX_TRACKS};

/// Bumped when a preset JSON layout changes; older files stay loadable only
/// via explicit migration (none needed at v1).
pub const PRESET_FORMAT_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Preset kinds
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetKind {
    Instrument,
    Pattern,
    Grid,
    Song,
}

impl PresetKind {
    pub const ALL: [Self; 4] = [Self::Instrument, Self::Pattern, Self::Grid, Self::Song];

    pub fn label(self) -> &'static str {
        match self {
            Self::Instrument => "Instruments",
            Self::Pattern => "Patterns",
            Self::Grid => "Grid",
            Self::Song => "Songs",
        }
    }

    fn subdir(self) -> &'static str {
        match self {
            Self::Instrument => "instruments",
            Self::Pattern => "patterns",
            Self::Grid => "grids",
            Self::Song => "songs",
        }
    }

    /// Files end with e.g. `.fdpat.json`.
    fn extension(self) -> &'static str {
        match self {
            Self::Instrument => "fdinst.json",
            Self::Pattern => "fdpat.json",
            Self::Grid => "fdgrid.json",
            Self::Song => "fdsong.json",
        }
    }
}

// ---------------------------------------------------------------------------
// Preset payloads
// ---------------------------------------------------------------------------

/// One instrument's sound (a slot's standards + specials + algo).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InstrumentPreset {
    pub version: u32,
    pub name: String,
    /// `TrackInstrumentKind` index — a preset only applies to a lane of the
    /// same kind (the UI offers to switch the lane's kind on load).
    pub kind: usize,
    /// 13 standard fields in registry order: freq, decay, volume, filter_freq,
    /// attack, release, decay_curve, release_curve, hold, filter_env_amount,
    /// filter_env_decay, analog, stereo.
    pub standards: [f32; 13],
    pub algo: u8,
    /// Special values in the registry's `special_params` order for the kind.
    pub specials: Vec<f32>,
}

/// One active lane's sound (standards + specials + algo) captured with a
/// pattern, keyed by slot so it can be re-applied to the right lane on load.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PatternSlotSound {
    /// Slot index (0..MAX_TRACKS) this sound belongs to.
    pub slot: u8,
    /// `TrackInstrumentKind` index at capture time — the sound is only
    /// re-applied to a slot that still holds this kind.
    pub kind: usize,
    /// 13 standard fields in registry order (see `InstrumentPreset`).
    pub standards: [f32; 13],
    pub algo: u8,
    /// Special values in the kind's `special_params` order.
    pub specials: Vec<f32>,
}

/// A full pattern: grid + fusions + sound plocks + seq plocks, plus the lane
/// kit it was captured with (applied only if the user asks for it on load) and
/// each active lane's instrument sound.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PatternPreset {
    pub version: u32,
    pub name: String,
    /// Lane kinds at capture time: `TrackInstrumentKind` index, or -1 for an
    /// inactive slot.
    pub kit: [i8; MAX_TRACKS],
    #[serde(with = "serde_arrays")]
    pub step_masks: [u16; 64],
    /// Hex-encoded binary blobs (same layout as the pattern bank slots, so
    /// legacy-length tolerance applies on load).
    pub plock_hex: String,
    pub seq_plock_hex: String,
    pub fusion_hex: String,
    pub pattern_length: u8,
    /// Per-lane instrument sounds captured with the pattern. `#[serde(default)]`
    /// so presets saved before this field load as an empty list (grid-only,
    /// sounds left untouched on apply).
    #[serde(default)]
    pub sounds: Vec<PatternSlotSound>,
}

/// A song sequence (16 blocks + repeats).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SongPreset {
    pub version: u32,
    pub name: String,
    pub song: SongSequence,
}

/// A lane kit only (which instrument sits on which slot) — the "Grid" preset
/// type, generalising the old fixed lane presets (4/12 lanes).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GridPreset {
    pub version: u32,
    pub name: String,
    /// `TrackInstrumentKind` index per slot, or -1 for an inactive slot.
    pub kit: [i8; MAX_TRACKS],
}

// ---------------------------------------------------------------------------
// Capture helpers (live state -> preset)
// ---------------------------------------------------------------------------

/// Capture a slot's sound. `algo` is the slot's algorithm index.
pub fn capture_instrument(
    name: String,
    kind: TrackInstrumentKind,
    inst: &InstrumentSettingsState,
    algo: i32,
) -> InstrumentPreset {
    use std::sync::atomic::Ordering;
    let g = |a: &std::sync::atomic::AtomicU32| f32::from_bits(a.load(Ordering::Relaxed));
    let standards = [
        g(&inst.frequency),
        g(&inst.decay),
        g(&inst.volume),
        g(&inst.filter_freq),
        g(&inst.attack),
        g(&inst.release),
        g(&inst.decay_curve),
        g(&inst.release_curve),
        g(&inst.hold),
        g(&inst.filter_env_amount),
        g(&inst.filter_env_decay),
        g(&inst.analog),
        g(&inst.stereo),
    ];
    let inst_def = kind.instrument_def();
    let specials = inst_def
        .special_params
        .iter()
        .map(|def| inst.special_value(def.special_index))
        .collect();
    InstrumentPreset {
        version: PRESET_FORMAT_VERSION,
        name,
        kind: kind.index(),
        standards,
        algo: algo.clamp(0, 255) as u8,
        specials,
    }
}

/// Capture the current pattern (grid + fusions + plocks + seq plocks), the
/// current lane kit, and each active lane's instrument sound. `algos[i]` is the
/// algorithm index of slot `i`.
#[allow(clippy::too_many_arguments)]
pub fn capture_pattern(
    name: String,
    layout: &crate::track::TrackLayoutState,
    pattern: &SharedPattern,
    plock_state: &crate::plock::PlockState,
    seq_plock_state: &crate::plock::SequencerPlockState,
    pattern_length: u8,
    sound_settings: &SoundSettingsState,
    algos: &[i32],
) -> PatternPreset {
    let mut slot = PatternSlot::default();
    slot.capture(pattern, plock_state, seq_plock_state, pattern_length);
    let mut kit = [-1i8; MAX_TRACKS];
    let mut sounds = Vec::new();
    for (i, s) in layout.slots.iter().enumerate() {
        if s.active {
            kit[i] = s.kind.index() as i8;
            // Reuse the instrument-sound extraction so the two paths stay in sync.
            let algo = algos.get(i).copied().unwrap_or(0);
            let ip = capture_instrument(String::new(), s.kind, &sound_settings.instruments[i], algo);
            sounds.push(PatternSlotSound {
                slot: i as u8,
                kind: ip.kind,
                standards: ip.standards,
                algo: ip.algo,
                specials: ip.specials,
            });
        }
    }
    PatternPreset {
        version: PRESET_FORMAT_VERSION,
        name,
        kit,
        step_masks: slot.step_masks,
        plock_hex: hex_encode(&slot.plock_bytes),
        seq_plock_hex: hex_encode(&slot.seq_plock_bytes),
        fusion_hex: hex_encode(&slot.fusion_bytes),
        pattern_length,
        sounds,
    }
}

pub fn capture_song(name: String, song: SongSequence) -> SongPreset {
    SongPreset {
        version: PRESET_FORMAT_VERSION,
        name,
        song,
    }
}

/// Capture the current lane kit as a Grid preset.
pub fn capture_grid(name: String, layout: &crate::track::TrackLayoutState) -> GridPreset {
    let mut kit = [-1i8; MAX_TRACKS];
    for (i, s) in layout.slots.iter().enumerate() {
        if s.active {
            kit[i] = s.kind.index() as i8;
        }
    }
    GridPreset {
        version: PRESET_FORMAT_VERSION,
        name,
        kit,
    }
}

/// Build a layout from a kit array (`-1` = inactive slot).
pub fn layout_from_kit(kit: &[i8; MAX_TRACKS]) -> crate::track::TrackLayoutState {
    let mut layout = crate::track::TrackLayoutState::empty_layout();
    for (i, k) in kit.iter().enumerate() {
        if *k >= 0 {
            if let Some(kind) = TrackInstrumentKind::from_index(*k as usize) {
                layout.slots[i] = crate::track::TrackSlot::active_with_kind(kind);
            }
        }
    }
    layout
}

// ---------------------------------------------------------------------------
// File storage
// ---------------------------------------------------------------------------

fn presets_root() -> PathBuf {
    let mut p = std::env::var("USERPROFILE")
        .map(|profile| PathBuf::from(profile).join("Documents"))
        .unwrap_or_else(|_| PathBuf::from("."));
    p.push("Flash Drum");
    p.push("presets");
    p
}

pub fn presets_dir(kind: PresetKind) -> PathBuf {
    presets_root().join(kind.subdir())
}

/// `Documents/Flash Drum/presets/_factory/<subdir>/` — the staging area the
/// debug "Export factory" button writes to; files are then committed under
/// `assets/presets/` to be embedded in the binary.
#[cfg(debug_assertions)]
pub fn factory_staging_dir(kind: PresetKind) -> PathBuf {
    presets_root().join("_factory").join(kind.subdir())
}

fn sanitize_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() {
        "preset".to_string()
    } else {
        s
    }
}

/// Info about a preset file on disk (name from content, path for load/delete).
#[derive(Debug, Clone)]
pub struct PresetFileInfo {
    pub name: String,
    pub path: PathBuf,
}

fn save_json(dir: PathBuf, name: &str, kind: PresetKind, json: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.{}", sanitize_name(name), kind.extension()));
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path)
}

fn load_json(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

pub fn delete_file(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| e.to_string())
}

// -- Typed save/load/list ----------------------------------------------------

pub fn save_instrument(p: &InstrumentPreset) -> Result<PathBuf, String> {
    save_json(
        presets_dir(PresetKind::Instrument),
        &p.name,
        PresetKind::Instrument,
        &serde_json::to_string_pretty(p).map_err(|e| e.to_string())?,
    )
}

pub fn save_pattern(p: &PatternPreset) -> Result<PathBuf, String> {
    save_json(
        presets_dir(PresetKind::Pattern),
        &p.name,
        PresetKind::Pattern,
        &serde_json::to_string_pretty(p).map_err(|e| e.to_string())?,
    )
}

pub fn save_song(p: &SongPreset) -> Result<PathBuf, String> {
    save_json(
        presets_dir(PresetKind::Song),
        &p.name,
        PresetKind::Song,
        &serde_json::to_string_pretty(p).map_err(|e| e.to_string())?,
    )
}

pub fn save_grid(p: &GridPreset) -> Result<PathBuf, String> {
    save_json(
        presets_dir(PresetKind::Grid),
        &p.name,
        PresetKind::Grid,
        &serde_json::to_string_pretty(p).map_err(|e| e.to_string())?,
    )
}

/// List user presets of a kind, sorted by name.
pub fn list_presets(kind: PresetKind) -> Vec<PresetFileInfo> {
    let dir = presets_dir(kind);
    list_dir(&dir, kind)
}

fn list_dir(dir: &Path, kind: PresetKind) -> Vec<PresetFileInfo> {
    let suffix = format!(".{}", kind.extension());
    let mut infos = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_preset = path
                .file_name()
                .and_then(|s| s.to_str())
                .map_or(false, |s| s.ends_with(&suffix));
            if !is_preset {
                continue;
            }
            // Read the display name from the file content (fall back to the
            // file stem if unparseable).
            let name = load_json(&path)
                .ok()
                .and_then(|content| {
                    serde_json::from_str::<NameOnly>(&content)
                        .ok()
                        .map(|n| n.name)
                })
                .unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("preset")
                        .trim_end_matches(match kind {
                            PresetKind::Instrument => ".fdinst",
                            PresetKind::Pattern => ".fdpat",
                            PresetKind::Grid => ".fdgrid",
                            PresetKind::Song => ".fdsong",
                        })
                        .to_string()
                });
            infos.push(PresetFileInfo { name, path });
        }
    }
    infos.sort_by(|a, b| a.name.cmp(&b.name));
    infos
}

#[derive(Deserialize)]
struct NameOnly {
    name: String,
}

// ---------------------------------------------------------------------------
// Factory presets (embedded) + debug export
// ---------------------------------------------------------------------------

/// Parse the embedded factory presets of a kind (name + payload JSON).
pub fn factory_presets(kind: PresetKind) -> Vec<(String, &'static str)> {
    let raw: &[&str] = match kind {
        PresetKind::Instrument => crate::factory_presets::INSTRUMENTS,
        PresetKind::Pattern => crate::factory_presets::PATTERNS,
        PresetKind::Grid => crate::factory_presets::GRIDS,
        PresetKind::Song => crate::factory_presets::SONGS,
    };
    raw.iter()
        .filter_map(|json| {
            serde_json::from_str::<NameOnly>(json)
                .ok()
                .map(|n| (n.name, *json))
        })
        .collect()
}

/// Debug-only factory authoring: write a preset into the `_factory` staging
/// directory (then copied into `assets/presets/` and committed).
#[cfg(debug_assertions)]
pub fn export_factory(kind: PresetKind, name: &str, json: &str) -> Result<PathBuf, String> {
    save_json(factory_staging_dir(kind), name, kind, json)
}

// ---------------------------------------------------------------------------
// Instrument presets filtered by kind (for the Track-tab quick loader)
// ---------------------------------------------------------------------------

/// Where an instrument preset comes from.
#[derive(Debug, Clone)]
pub enum InstrumentPresetSource {
    /// Embedded factory preset (its JSON payload).
    Factory(&'static str),
    /// User preset file on disk.
    User(PathBuf),
}

/// A selectable instrument preset (display name + how to load it).
#[derive(Debug, Clone)]
pub struct InstrumentPresetEntry {
    pub name: String,
    pub source: InstrumentPresetSource,
}

/// Just the `kind` field, to filter instrument presets without a full parse.
#[derive(Deserialize)]
struct KindOnly {
    kind: usize,
}

/// List every instrument preset (factory first, then user) that applies to the
/// given `TrackInstrumentKind` index, sorted factory-then-user, each group by
/// name. Reads/parses files, so callers should cache the result rather than
/// call it every frame.
pub fn list_instrument_presets(kind_index: usize) -> Vec<InstrumentPresetEntry> {
    let mut out = Vec::new();
    for json in crate::factory_presets::INSTRUMENTS {
        if let Ok(k) = serde_json::from_str::<KindOnly>(json) {
            if k.kind == kind_index {
                let name = serde_json::from_str::<NameOnly>(json)
                    .map(|n| n.name)
                    .unwrap_or_else(|_| "preset".to_string());
                out.push(InstrumentPresetEntry {
                    name,
                    source: InstrumentPresetSource::Factory(json),
                });
            }
        }
    }
    let factory_end = out.len();
    for info in list_presets(PresetKind::Instrument) {
        if let Ok(content) = load_json(&info.path) {
            if let Ok(k) = serde_json::from_str::<KindOnly>(&content) {
                if k.kind == kind_index {
                    out.push(InstrumentPresetEntry {
                        name: info.name,
                        source: InstrumentPresetSource::User(info.path),
                    });
                }
            }
        }
    }
    out[factory_end..].sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Resolve an entry to a full `InstrumentPreset` (parses the factory JSON or
/// reads the user file). Returns `None` on any I/O or parse error.
pub fn load_instrument_preset(entry: &InstrumentPresetEntry) -> Option<InstrumentPreset> {
    let json = match &entry.source {
        InstrumentPresetSource::Factory(j) => (*j).to_string(),
        InstrumentPresetSource::User(p) => load_json(p).ok()?,
    };
    serde_json::from_str::<InstrumentPreset>(&json).ok()
}

// ---------------------------------------------------------------------------
// Hex helpers (binary blobs inside JSON)
// ---------------------------------------------------------------------------

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xF) as usize] as char);
    }
    out
}

pub fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let bytes = s.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err("odd hex length".to_string());
    }
    let val = |c: u8| -> Result<u8, String> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(format!("invalid hex char {}", c as char)),
        }
    };
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        out.push((val(pair[0])? << 4) | val(pair[1])?);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let data: Vec<u8> = (0..=255).collect();
        assert_eq!(hex_decode(&hex_encode(&data)).unwrap(), data);
        assert!(hex_decode("abc").is_err()); // odd length
        assert!(hex_decode("zz").is_err()); // invalid char
    }

    #[test]
    fn sanitize_name_strips_specials() {
        assert_eq!(sanitize_name("My Preset!"), "My_Preset");
        assert_eq!(sanitize_name("___"), "preset");
    }

    #[test]
    fn instrument_preset_json_roundtrip() {
        let p = InstrumentPreset {
            version: PRESET_FORMAT_VERSION,
            name: "Punchy".to_string(),
            kind: TrackInstrumentKind::Kick.index(),
            standards: [1.0; 13],
            algo: 2,
            specials: vec![0.5, 1.0, 3.0],
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: InstrumentPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn pattern_preset_capture_has_decodable_blobs() {
        let pattern = SharedPattern::new(&crate::sequencer::pattern::Pattern::empty());
        pattern.set_step_mask(0, 0b101);
        let plock = crate::plock::PlockState::new();
        let seq = crate::plock::SequencerPlockState::new();
        let layout = crate::track::TrackLayoutState::modular_default_layout();
        let sound = SoundSettingsState::new(&layout);
        let algos = [0i32; MAX_TRACKS];

        let p = capture_pattern(
            "Test".into(),
            &layout,
            &pattern,
            &plock,
            &seq,
            32,
            &sound,
            &algos,
        );
        assert_eq!(p.step_masks[0], 0b101);
        assert!(hex_decode(&p.plock_hex).is_ok());
        assert!(hex_decode(&p.seq_plock_hex).is_ok());
        assert!(hex_decode(&p.fusion_hex).is_ok());
        assert_eq!(p.pattern_length, 32);
        // Kit: 4 active lanes (Kick/Snare/HiHat/Tom), slot 4 inactive.
        assert_eq!(p.kit[0], TrackInstrumentKind::Kick.index() as i8);
        assert_eq!(p.kit[3], TrackInstrumentKind::Tom.index() as i8);
        assert_eq!(p.kit[4], -1);
        // Each active lane captured its instrument sound.
        assert_eq!(p.sounds.len(), 4);
        assert_eq!(p.sounds[0].slot, 0);
        assert_eq!(p.sounds[0].kind, TrackInstrumentKind::Kick.index());

        let json = serde_json::to_string(&p).unwrap();
        let back: PatternPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(back.step_masks[0], 0b101);
        assert_eq!(back.kit, p.kit);
        assert_eq!(back.sounds, p.sounds);
    }

    #[test]
    fn pattern_preset_without_sounds_field_loads_empty() {
        // A pattern preset saved before the `sounds` field existed must still
        // deserialize (serde default → empty list, sounds left untouched on apply).
        let json = r#"{"version":1,"name":"old","kit":[-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1,-1],"step_masks":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"plock_hex":"","seq_plock_hex":"","fusion_hex":"","pattern_length":16}"#;
        let back: PatternPreset = serde_json::from_str(json).unwrap();
        assert!(back.sounds.is_empty());
        assert_eq!(back.pattern_length, 16);
    }
}
