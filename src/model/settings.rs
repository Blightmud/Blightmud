use crate::io::SaveData;
use anyhow::bail;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Settings {
    settings: HashMap<String, bool>,
}

pub const LOGGING_ENABLED: &str = "logging_enabled";
pub const TTS_ENABLED: &str = "tts_enabled";
pub const MOUSE_ENABLED: &str = "mouse_enabled";
pub const SAVE_HISTORY: &str = "save_history";
pub const CONFIRM_QUIT: &str = "confirm_quit";
pub const SCROLL_SPLIT: &str = "scroll_split";
pub const SCROLL_LOCK: &str = "scroll_lock";
pub const READER_MODE: &str = "reader_mode";
pub const HIDE_TOPBAR: &str = "hide_topbar";
pub const COMMAND_SEARCH: &str = "command_search";
pub const SMART_HISTORY: &str = "smart_history";
pub const ECHO_INPUT: &str = "echo_input";

pub const KEEPALIVE_ENABLED: &str = "keepalive_enabled";

pub const SETTINGS: [&str; 13] = [
    LOGGING_ENABLED,
    TTS_ENABLED,
    MOUSE_ENABLED,
    SAVE_HISTORY,
    CONFIRM_QUIT,
    SCROLL_SPLIT,
    SCROLL_LOCK,
    READER_MODE,
    HIDE_TOPBAR,
    COMMAND_SEARCH,
    SMART_HISTORY,
    ECHO_INPUT,
    KEEPALIVE_ENABLED,
];

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
        settings.insert(SAVE_HISTORY.to_string(), false);
        settings.insert(CONFIRM_QUIT.to_string(), true);
        settings.insert(SCROLL_SPLIT.to_string(), true);
        settings.insert(SCROLL_LOCK.to_string(), true);
        settings.insert(READER_MODE.to_string(), false);
        settings.insert(HIDE_TOPBAR.to_string(), false);
        settings.insert(COMMAND_SEARCH.to_string(), false);
        settings.insert(SMART_HISTORY.to_string(), false);
        settings.insert(ECHO_INPUT.to_string(), true);
        settings.insert(KEEPALIVE_ENABLED.to_string(), true);
        Self { settings }
    }
}

impl SaveData for Settings {
    fn relative_path() -> std::path::PathBuf {
        crate::CONFIG_DIR.join("settings.ron")
    }

    fn on_load(&mut self) {
        let default = Self::default();
        if default.settings.len() != self.settings.len() {
            for (key, val) in default.settings {
                self.settings.entry(key).or_insert(val);
            }
        }
    }

    fn is_pretty() -> bool {
        true
    }
}

impl From<HashMap<String, bool>> for Settings {
    fn from(map: HashMap<String, bool>) -> Self {
        Self { settings: map }
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
