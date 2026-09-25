//! [260] Single source of truth for the user-facing folder
//! (`Documents/Flash Drum`), replacing four hand-rolled
//! `%USERPROFILE%\Documents` copies (config, presets, preset dumps, MIDI
//! exports) that missed OneDrive's Known Folder Move — the real Documents is
//! then `%USERPROFILE%\OneDrive\Documents` — and did not exist on macOS at
//! all (no `USERPROFILE`, so data landed in the host's CWD, often `/`).

use std::path::PathBuf;

/// `Documents/Flash Drum` — the real Documents folder, falling back to the
/// home directory, then to the temp dir (always writable).
pub fn flash_drum_dir() -> PathBuf {
    static MIGRATE: std::sync::Once = std::sync::Once::new();
    let dir = dirs::document_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("Flash Drum");
    MIGRATE.call_once(|| migrate_legacy_dir(&dir));
    dir
}

/// One-time migration from the pre-[260] location. With OneDrive's Known
/// Folder Move, `%USERPROFILE%\Documents\Flash Drum` is a ghost folder the
/// user never sees; if it exists and the new folder doesn't, copy it over.
/// The legacy folder is never deleted (the copy is best-effort).
#[cfg(target_os = "windows")]
fn migrate_legacy_dir(new_dir: &std::path::Path) {
    let Some(legacy) = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|p| p.join("Documents").join("Flash Drum"))
    else {
        return;
    };
    // Same folder when OneDrive is off: nothing to do. `new_dir.exists()`
    // also covers a string mismatch (separators/case) between the two.
    if legacy == new_dir || !legacy.is_dir() || new_dir.exists() {
        return;
    }
    let _ = copy_dir_all(&legacy, new_dir);
}

#[cfg(not(target_os = "windows"))]
fn migrate_legacy_dir(_new_dir: &std::path::Path) {}

#[cfg(target_os = "windows")]
fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            let _ = copy_dir_all(&entry.path(), &target);
        } else {
            let _ = std::fs::copy(entry.path(), &target);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_dirs_share_one_root() {
        let root = flash_drum_dir();
        assert!(root.is_absolute());
        assert!(root.ends_with("Flash Drum"));
        assert!(crate::config::GlobalConfig::config_path().starts_with(&root));
        assert!(crate::presets::presets_root().starts_with(&root));
        assert!(crate::preset_dumps::dumps_dir().starts_with(&root));
        assert!(crate::ui::midi::exports_dir().starts_with(&root));
    }
}
