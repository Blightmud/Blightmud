use crate::io::SaveData;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use simple_error::bail;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    settings: HashMap<String, bool>,
}

pub const LOGGING_ENABLED: &str = "logging_enabled";
pub const TTS_ENABLED: &str = "tts_enabled";
pub const SETTINGS: [&str; 2] = [LOGGING_ENABLED, TTS_ENABLED];

impl Settings {
    pub fn get(&self, key: &str) -> Result<bool> {
        if SETTINGS.contains(&key) {
            Ok(*self.settings.get(key).unwrap_or(&false))
        } else {
            bail!("Unknown setting: {}", key)
        }
    }

    pub fn set(&mut self, key: &str, value: bool) -> Result<()> {
        if SETTINGS.contains(&key) {
            self.settings.insert(key.to_string(), value);
            Ok(())
        } else {
            bail!("Unknown setting: {}", key)
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let mut settings: HashMap<String, bool> = HashMap::new();
        settings.insert(LOGGING_ENABLED.to_string(), false);
        settings.insert(TTS_ENABLED.to_string(), true);
        Self { settings }
    }
}

impl SaveData for Settings {
    fn relative_path() -> std::path::PathBuf {
        crate::DATA_DIR.join("config").join("settings.ron")
    }
}
