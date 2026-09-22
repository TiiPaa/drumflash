//! [228] One custom texture per lane for Rift: the persisted file paths and
//! the pool the voices read.
//!
//! Paths live in the session (`lane-textures-v1`, a JSON list of one optional
//! path per lane). On restore, `PersistentField::set` runs on the host's main
//! thread and decodes every file right there - never on the audio thread,
//! which only ever sees the pool. A file that has gone missing leaves its lane
//! empty and its path in place, so the UI can say so and the voice falls back
//! to the first embedded texture instead of going silent.
//!
//! Two lanes loading the SAME file share one decoded texture: a cache keyed by
//! path, modification time and size hands the second lane the first one's
//! `Arc`. A file re-exported under the same name has a new mtime and is
//! decoded again.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, Weak};

use nih_plug::params::persist::PersistentField;

use crate::synthesis::sample_bank::{load_texture_file, TextureBank, TexturePool, LANE_TEXTURE_SLOTS};

/// What the Sound Panel shows for a lane.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlotStatus {
    /// No file assigned: "Custom" plays the first embedded texture.
    Empty,
    Loaded,
    /// A path is remembered but the file could not be read (moved, deleted,
    /// or not a WAV): "Custom" plays the first embedded texture.
    Missing,
}

/// Identity of a file on disk for the sharing cache.
#[derive(Clone, PartialEq, Eq, Hash)]
struct FileKey {
    path: String,
    modified_secs: u64,
    len: u64,
}

impl FileKey {
    fn of(path: &Path) -> Option<Self> {
        let meta = std::fs::metadata(path).ok()?;
        let modified_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(Self {
            path: path.to_string_lossy().into_owned(),
            modified_secs,
            len: meta.len(),
        })
    }
}

pub struct UserTextures {
    paths: RwLock<Vec<Option<String>>>,
    errors: RwLock<Vec<Option<String>>>,
    /// Decoded files still alive somewhere, by identity on disk.
    cache: Mutex<HashMap<FileKey, Weak<TextureBank>>>,
    pub pool: Arc<TexturePool>,
}

impl Default for UserTextures {
    fn default() -> Self {
        Self::new()
    }
}

impl UserTextures {
    pub fn new() -> Self {
        Self {
            paths: RwLock::new(vec![None; LANE_TEXTURE_SLOTS]),
            errors: RwLock::new(vec![None; LANE_TEXTURE_SLOTS]),
            cache: Mutex::new(HashMap::new()),
            pool: Arc::new(TexturePool::new()),
        }
    }

    pub fn path(&self, lane: usize) -> Option<PathBuf> {
        self.paths
            .read()
            .ok()
            .and_then(|p| p.get(lane).cloned().flatten())
            .map(PathBuf::from)
    }

    /// The file's name without its folder, for the panel.
    pub fn file_name(&self, lane: usize) -> Option<String> {
        self.path(lane)
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    }

    pub fn status(&self, lane: usize) -> SlotStatus {
        match (self.path(lane).is_some(), self.pool.is_loaded(lane)) {
            (false, _) => SlotStatus::Empty,
            (true, true) => SlotStatus::Loaded,
            (true, false) => SlotStatus::Missing,
        }
    }

    /// Why a lane is `Missing`, when the decoder said.
    pub fn error(&self, lane: usize) -> Option<String> {
        self.errors
            .read()
            .ok()
            .and_then(|e| e.get(lane).cloned().flatten())
    }

    /// Decode `path` into `lane` (or share the texture another lane already
    /// decoded from the very same file) and remember the path. UI or main
    /// thread only. On failure the lane is emptied but the path is kept: that
    /// is the "missing" state the panel reports.
    pub fn load(&self, lane: usize, path: &Path) -> Result<(), String> {
        if lane >= LANE_TEXTURE_SLOTS {
            return Err("no such lane".to_string());
        }
        if let Ok(mut paths) = self.paths.write() {
            paths[lane] = Some(path.to_string_lossy().into_owned());
        }
        let result = self.decode_shared(path);
        match result {
            Ok(bank) => {
                self.pool.publish(lane, Some(bank));
                if let Ok(mut errors) = self.errors.write() {
                    errors[lane] = None;
                }
                Ok(())
            }
            Err(e) => {
                self.pool.publish(lane, None);
                if let Ok(mut errors) = self.errors.write() {
                    errors[lane] = Some(e.clone());
                }
                Err(e)
            }
        }
    }

    /// The cache lookup: a live texture decoded from this exact file, or a
    /// fresh decode registered for the next lane to share.
    fn decode_shared(&self, path: &Path) -> Result<Arc<TextureBank>, String> {
        let key = FileKey::of(path);
        if let (Some(key), Ok(mut cache)) = (key.as_ref(), self.cache.lock()) {
            if let Some(live) = cache.get(key).and_then(Weak::upgrade) {
                return Ok(live);
            }
            // Drop dead entries while we are here.
            cache.retain(|_, weak| weak.strong_count() > 0);
        }
        let bank = Arc::new(load_texture_file(path)?);
        if let (Some(key), Ok(mut cache)) = (key, self.cache.lock()) {
            cache.insert(key, Arc::downgrade(&bank));
        }
        Ok(bank)
    }

    /// Forget the lane's file: "Custom" plays the first embedded texture again.
    pub fn clear(&self, lane: usize) {
        if lane >= LANE_TEXTURE_SLOTS {
            return;
        }
        self.pool.publish(lane, None);
        if let Ok(mut paths) = self.paths.write() {
            paths[lane] = None;
        }
        if let Ok(mut errors) = self.errors.write() {
            errors[lane] = None;
        }
    }

    /// Permute the lanes: `order[new] = old`, as the lane drag-and-drop does
    /// for every other per-lane store. The decoded textures move with their
    /// lane (the same `Arc`s, republished under their new numbers), so a
    /// moved lane keeps its file and nothing is decoded again.
    pub fn reorder(&self, order: &[usize]) {
        let old_paths: Vec<Option<String>> = self.map(|p| p.clone());
        let old_errors: Vec<Option<String>> = self
            .errors
            .read()
            .map(|e| e.clone())
            .unwrap_or_else(|_| vec![None; LANE_TEXTURE_SLOTS]);
        let old_banks: Vec<Option<Arc<TextureBank>>> =
            (0..LANE_TEXTURE_SLOTS).map(|lane| self.pool.get(lane)).collect();
        let mut new_paths = vec![None; LANE_TEXTURE_SLOTS];
        let mut new_errors = vec![None; LANE_TEXTURE_SLOTS];
        for (new_idx, &old_idx) in order.iter().enumerate().take(LANE_TEXTURE_SLOTS) {
            new_paths[new_idx] = old_paths.get(old_idx).cloned().flatten();
            new_errors[new_idx] = old_errors.get(old_idx).cloned().flatten();
            self.pool
                .publish(new_idx, old_banks.get(old_idx).cloned().flatten());
        }
        if let Ok(mut paths) = self.paths.write() {
            *paths = new_paths;
        }
        if let Ok(mut errors) = self.errors.write() {
            *errors = new_errors;
        }
    }

    /// The lane's file is stereo (two channels kept at decode).
    pub fn is_stereo(&self, lane: usize) -> bool {
        self.pool
            .get(lane)
            .map(|b| b.right.is_some())
            .unwrap_or(false)
    }

    fn reload_all(&self) {
        let paths: Vec<Option<String>> = self
            .paths
            .read()
            .map(|p| p.clone())
            .unwrap_or_default();
        for (lane, path) in paths.iter().enumerate().take(LANE_TEXTURE_SLOTS) {
            match path {
                Some(p) => {
                    let _ = self.load(lane, Path::new(p));
                }
                None => self.clear(lane),
            }
        }
    }
}

impl<'a> PersistentField<'a, Vec<Option<String>>> for UserTextures {
    fn set(&self, new_value: Vec<Option<String>>) {
        let mut paths = new_value;
        paths.resize(LANE_TEXTURE_SLOTS, None);
        if let Ok(mut current) = self.paths.write() {
            *current = paths;
        }
        // Main thread (state restore): decode now, the audio thread only ever
        // sees the pool.
        self.reload_all();
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&Vec<Option<String>>) -> R,
    {
        match self.paths.read() {
            Ok(paths) => f(&paths),
            Err(_) => f(&vec![None; LANE_TEXTURE_SLOTS]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_wav(path: &Path, frames: usize) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..frames {
            w.write_sample(((i % 50) as i32 - 25) * 100).unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn missing_file_keeps_its_path_and_reports_missing() {
        let textures = UserTextures::new();
        assert_eq!(textures.status(0), SlotStatus::Empty);
        let gone = std::env::temp_dir().join("flash-drum-definitely-missing.wav");
        assert!(textures.load(0, &gone).is_err());
        assert_eq!(textures.status(0), SlotStatus::Missing);
        assert!(textures.file_name(0).unwrap().ends_with("missing.wav"));
        assert!(textures.error(0).is_some());
        assert!(!textures.pool.is_loaded(0));
        textures.clear(0);
        assert_eq!(textures.status(0), SlotStatus::Empty);
    }

    #[test]
    fn restore_reloads_the_listed_paths() {
        let dir = std::env::temp_dir().join(format!("flash-drum-ut-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("tex.wav");
        write_test_wav(&file, 800);

        let textures = UserTextures::new();
        let mut paths = vec![None; LANE_TEXTURE_SLOTS];
        paths[3] = Some(file.to_string_lossy().into_owned());
        paths[5] = Some(dir.join("nope.wav").to_string_lossy().into_owned());
        textures.set(paths);
        assert_eq!(textures.status(3), SlotStatus::Loaded);
        assert_eq!(textures.status(5), SlotStatus::Missing);
        assert_eq!(textures.status(0), SlotStatus::Empty);
        // What is persisted is what was set.
        let round: Vec<Option<String>> = textures.map(|p| p.clone());
        assert_eq!(round.len(), LANE_TEXTURE_SLOTS);
        assert!(round[3].as_deref().unwrap().ends_with("tex.wav"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Moving lanes moves their files with them, without decoding again.
    #[test]
    fn reorder_moves_the_texture_with_its_lane() {
        let dir = std::env::temp_dir().join(format!("flash-drum-reorder-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("moved.wav");
        write_test_wav(&file, 800);

        let textures = UserTextures::new();
        textures.load(1, &file).unwrap();
        let before = textures.pool.get(1).unwrap();
        // Lane 1 moves to lane 3: order[new] = old, lanes 2 and 3 shift up.
        let mut order: Vec<usize> = (0..LANE_TEXTURE_SLOTS).collect();
        order[1] = 2;
        order[2] = 3;
        order[3] = 1;
        textures.reorder(&order);
        assert_eq!(textures.status(1), SlotStatus::Empty);
        assert_eq!(textures.status(3), SlotStatus::Loaded);
        assert!(textures.file_name(3).unwrap().ends_with("moved.wav"));
        let after = textures.pool.get(3).unwrap();
        assert!(Arc::ptr_eq(&before, &after), "same decoded texture, just renumbered");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Two lanes on the same file share one decoded texture; a rewritten
    /// file (new size) is decoded again.
    #[test]
    fn lanes_loading_the_same_file_share_one_texture() {
        let dir = std::env::temp_dir().join(format!("flash-drum-share-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("shared.wav");
        write_test_wav(&file, 800);

        let textures = UserTextures::new();
        textures.load(1, &file).unwrap();
        textures.load(4, &file).unwrap();
        let a = textures.pool.get(1).unwrap();
        let b = textures.pool.get(4).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "same file, same decoded texture");

        // Re-exported (longer) under the same name: a new decode.
        write_test_wav(&file, 1600);
        textures.load(7, &file).unwrap();
        let c = textures.pool.get(7).unwrap();
        assert!(!Arc::ptr_eq(&a, &c), "a changed file must be decoded again");
        assert_eq!(c.data.len(), 1600);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
