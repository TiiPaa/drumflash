//! MIDI file export and the external drag-and-drop helper (Windows).

use crate::midi_export;
use crate::sequencer::SharedPattern;
use std::fs::create_dir_all;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::process::Command;

pub fn export_midi_to_documents(
    pattern: &SharedPattern,
    track_layout: &crate::track::AtomicTrackLayout,
    bpm: f32,
    pattern_length: usize,
    swing: f32,
    groove_type: crate::groove::GrooveType,
    seq_plock: &crate::plock::SequencerPlockState,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let docs = std::env::var("USERPROFILE")
        .ok()
        .map(PathBuf::from)
        .map(|p| p.join("Documents"))
        .ok_or("Cannot find Documents folder")?;
    let export_dir = docs.join("Flash Drum").join("exports");
    create_dir_all(&export_dir)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let filename = format!("drum_pattern_{:.0}bpm_{}.mid", bpm, timestamp);
    let path = export_dir.join(filename);

    midi_export::export_pattern_to_midi(
        pattern,
        track_layout,
        bpm,
        pattern_length,
        swing,
        groove_type,
        seq_plock,
        &path,
    )?;
    Ok(path)
}

#[cfg(target_os = "windows")]
pub fn start_external_midi_drag(
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let helper = find_midi_drag_helper().ok_or("MIDI drag helper not found")?;
    Command::new(helper).arg(path).spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn helper_bundle_prefixes() -> Vec<PathBuf> {
    let mut prefixes = Vec::new();
    if let Ok(common_files) = std::env::var("CommonProgramFiles") {
        prefixes.push(
            PathBuf::from(common_files)
                .join("VST3")
                .join("drum-pattern-vst.vst3"),
        );
    }
    prefixes.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("build")
            .join("drum-pattern-vst.vst3"),
    );
    prefixes
}

/// Validate a helper executable candidate: the file must exist, carry the
/// expected helper name, and live under one of the known VST3 bundle prefixes
/// (checked on canonicalized paths so `..` traversal cannot escape).
#[cfg(target_os = "windows")]
fn is_valid_helper_candidate(path: &std::path::Path, prefixes: &[PathBuf]) -> bool {
    const HELPER_NAME: &str = "drum-pattern-midi-drag-helper.exe";
    if !path.is_file() {
        return false;
    }
    if path.file_name().map(|n| n != HELPER_NAME).unwrap_or(true) {
        return false;
    }
    let Ok(canonical) = path.canonicalize() else {
        return false;
    };
    prefixes.iter().any(|prefix| {
        prefix
            .canonicalize()
            .map(|p| canonical.starts_with(p))
            .unwrap_or(false)
    })
}

#[cfg(target_os = "windows")]
fn find_midi_drag_helper() -> Option<PathBuf> {
    const HELPER_NAME: &str = "drum-pattern-midi-drag-helper.exe";
    let prefixes = helper_bundle_prefixes();

    // The env override must point at the real helper inside a known bundle,
    // never at an arbitrary executable.
    if let Ok(path) = std::env::var("DRUM_FLASH_MIDI_DRAG_HELPER") {
        let path = PathBuf::from(path);
        if is_valid_helper_candidate(&path, &prefixes) {
            return Some(path);
        }
    }

    for prefix in &prefixes {
        let candidate = prefix
            .join("Contents")
            .join("x86_64-win")
            .join(HELPER_NAME);
        if is_valid_helper_candidate(&candidate, &prefixes) {
            return Some(candidate);
        }
    }

    None
}

#[cfg(not(target_os = "windows"))]
pub fn start_external_midi_drag(
    _path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("MIDI drag helper is only implemented on Windows".into())
}

#[cfg(all(test, target_os = "windows"))]
mod midi_drag_helper_tests {
    use super::*;
    use std::fs;

    fn make_helper(dir: &std::path::Path) -> PathBuf {
        let exe = dir
            .join("Contents")
            .join("x86_64-win")
            .join("drum-pattern-midi-drag-helper.exe");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, b"stub").unwrap();
        exe
    }

    #[test]
    fn accepts_helper_inside_bundle_prefix() {
        let root = std::env::temp_dir().join("fd_helper_test_ok");
        let _ = fs::remove_dir_all(&root);
        let bundle = root.join("drum-pattern-vst.vst3");
        let exe = make_helper(&bundle);
        assert!(is_valid_helper_candidate(&exe, &[bundle]));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_wrong_filename_inside_bundle() {
        let root = std::env::temp_dir().join("fd_helper_test_name");
        let _ = fs::remove_dir_all(&root);
        let bundle = root.join("drum-pattern-vst.vst3");
        let evil = bundle.join("Contents").join("x86_64-win").join("evil.exe");
        fs::create_dir_all(evil.parent().unwrap()).unwrap();
        fs::write(&evil, b"stub").unwrap();
        assert!(!is_valid_helper_candidate(&evil, &[bundle]));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_helper_outside_bundle_prefix() {
        let root = std::env::temp_dir().join("fd_helper_test_outside");
        let _ = fs::remove_dir_all(&root);
        let bundle = root.join("drum-pattern-vst.vst3");
        let outside = root.join("elsewhere");
        let exe = make_helper(&outside);
        assert!(!is_valid_helper_candidate(&exe, &[bundle]));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_dotdot_traversal_escaping_prefix() {
        let root = std::env::temp_dir().join("fd_helper_test_traversal");
        let _ = fs::remove_dir_all(&root);
        let bundle = root.join("drum-pattern-vst.vst3");
        let outside = root.join("elsewhere");
        let _ = make_helper(&outside);
        let traversal = bundle
            .join("..")
            .join("elsewhere")
            .join("Contents")
            .join("x86_64-win")
            .join("drum-pattern-midi-drag-helper.exe");
        assert!(traversal.exists());
        assert!(!is_valid_helper_candidate(&traversal, &[bundle]));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_missing_file() {
        let root = std::env::temp_dir().join("fd_helper_test_missing");
        let _ = fs::remove_dir_all(&root);
        let bundle = root.join("drum-pattern-vst.vst3");
        let missing = bundle
            .join("Contents")
            .join("x86_64-win")
            .join("drum-pattern-midi-drag-helper.exe");
        assert!(!is_valid_helper_candidate(&missing, &[bundle]));
    }
}
