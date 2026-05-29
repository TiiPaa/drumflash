use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

/// A dumped preset for developer use (Phase 1 factory-preset authoring tool).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PresetDump {
    pub name: String,
    pub instrument_idx: usize,
    pub instrument_label: String,
    /// 13 standard fields in order: freq, decay, vol, filter_freq, attack,
    /// release, decay_curve, release_curve, hold, filter_env_amount,
    /// filter_env_decay, analog, stereo.
    pub standards: [f32; 13],
    pub algo: u8,
    /// Only the special slots actually used by this instrument.
    pub specials: Vec<f32>,
}

fn dumps_dir() -> PathBuf {
    let mut p = std::env::var("USERPROFILE")
        .map(|profile| PathBuf::from(profile).join("Documents"))
        .unwrap_or_else(|_| PathBuf::from("."));
    p.push("Drum Flash");
    p.push("preset_dumps");
    p
}

/// Ensure the dumps directory exists.
pub fn ensure_dumps_dir() -> PathBuf {
    let dir = dumps_dir();
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Save a preset dump to disk.
pub fn dump_preset(dump: &PresetDump) -> Result<PathBuf, String> {
    let dir = ensure_dumps_dir();
    let safe_name = dump
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>();
    let filename = format!("{}_{}.json", dump.instrument_label.to_lowercase(), safe_name);
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(dump).map_err(|e| e.to_string())?;
    let mut file = File::create(&path).map_err(|e| e.to_string())?;
    file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Information about an existing dump file (without loading full contents).
#[derive(Debug, Clone)]
pub struct PresetDumpInfo {
    pub name: String,
    pub instrument_idx: usize,
    pub instrument_label: String,
    pub path: PathBuf,
}

/// List all dump files in the dumps directory.
pub fn list_dumps() -> Vec<PresetDumpInfo> {
    let dir = ensure_dumps_dir();
    let mut infos = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(dump) = serde_json::from_str::<PresetDump>(&content) {
                        infos.push(PresetDumpInfo {
                            name: dump.name.clone(),
                            instrument_idx: dump.instrument_idx,
                            instrument_label: dump.instrument_label,
                            path,
                        });
                    }
                }
            }
        }
    }

    // Sort by instrument_idx then name for stable display.
    infos.sort_by(|a, b| {
        a.instrument_idx
            .cmp(&b.instrument_idx)
            .then_with(|| a.name.cmp(&b.name))
    });
    infos
}

/// Load a full PresetDump from a path.
pub fn load_dump(path: &Path) -> Result<PresetDump, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// Delete a dump file.
pub fn delete_dump(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| e.to_string())
}
