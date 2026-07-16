use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Global user preferences stored outside the DAW project.
///
/// Lives in `%USERPROFILE%/Documents/Flash Drum/config.json` so settings
/// survive across sessions and are shared between all plugin instances.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobalConfig {
    /// Default value for the `Analog` parameter when a slot is created or reset.
    pub default_analog: f32,
    /// Default global MIDI channel used for MIDI input/output (1-16).
    pub global_midi_channel: u8,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            default_analog: 0.5,
            global_midi_channel: 10,
        }
    }
}

impl GlobalConfig {
    /// Load the config from disk, creating a default file if it doesn't exist.
    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str::<GlobalConfig>(&contents) {
                // Clamp to a sensible range in case the file was hand-edited.
                return config.clamped();
            }
        }
        let config = GlobalConfig::default();
        let _ = config.save();
        config
    }

    /// Persist the current config to disk.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)
    }

    /// Path to the config file: `Documents/Flash Drum/config.json`.
    fn config_path() -> PathBuf {
        let mut p = std::env::var("USERPROFILE")
            .map(|profile| PathBuf::from(profile).join("Documents"))
            .unwrap_or_else(|_| PathBuf::from("."));
        p.push("Flash Drum");
        p.push("config.json");
        p
    }

    fn clamped(mut self) -> Self {
        self.default_analog = self.default_analog.clamp(0.0, 1.0);
        self.global_midi_channel = self.global_midi_channel.clamp(1, 16);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_analog_is_05() {
        let config = GlobalConfig::default();
        assert!((config.default_analog - 0.5).abs() < 1e-6);
        assert_eq!(config.global_midi_channel, 10);
    }

    #[test]
    fn clamped_rejects_out_of_range_values() {
        let mut high = GlobalConfig::default();
        high.default_analog = 2.5;
        assert_eq!(high.clamped().default_analog, 1.0);

        let mut low = GlobalConfig::default();
        low.default_analog = -1.0;
        assert_eq!(low.clamped().default_analog, 0.0);

        let mut chan = GlobalConfig::default();
        chan.global_midi_channel = 25;
        assert_eq!(chan.clamped().global_midi_channel, 16);

        let mut chan = GlobalConfig::default();
        chan.global_midi_channel = 0;
        assert_eq!(chan.clamped().global_midi_channel, 1);
    }
}
