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
pub const MOUSE_ENABLED: &str = "mouse_enabled";
pub const SETTINGS: [&str; 3] = [LOGGING_ENABLED, TTS_ENABLED, MOUSE_ENABLED];

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
        settings.insert(MOUSE_ENABLED.to_string(), false);
        Self { settings }
    }
}

impl SaveData for Settings {
    fn relative_path() -> std::path::PathBuf {
        crate::DATA_DIR.join("config").join("settings.ron")
    }
}

#[cfg(test)]
mod settings_test {
    use super::*;

    #[test]
    fn get_settings() {
        let settings = Settings::default();

        assert_eq!(true, settings.get(TTS_ENABLED).unwrap());
        assert_eq!(false, settings.get(LOGGING_ENABLED).unwrap());
        assert_eq!(
            "Unknown setting: SOMETHING_RANDOM",
            settings.get("SOMETHING_RANDOM").unwrap_err().to_string()
        );
    }

    #[test]
    fn new_settings() {
        let map = HashMap::new();
        let settings = Settings { settings: map };
        assert_eq!(false, settings.get(TTS_ENABLED).unwrap());
        assert_eq!(
            "Unknown setting: SOMETHING_RANDOM",
            settings.get("SOMETHING_RANDOM").unwrap_err().to_string()
        );
    }

    #[test]
    fn set_settings() {
        let mut settings = Settings::default();

        settings.set(TTS_ENABLED, false).unwrap();
        settings.set(LOGGING_ENABLED, true).unwrap();

        assert_eq!(false, settings.get(TTS_ENABLED).unwrap());
        assert_eq!(true, settings.get(LOGGING_ENABLED).unwrap());
        assert_eq!(
            "Unknown setting: SOMETHING_RANDOM",
            settings
                .set("SOMETHING_RANDOM", true)
                .unwrap_err()
                .to_string()
        );
    }
}
