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

impl Settings {
    pub fn get(&self, key: &str) -> Result<bool> {
        if let Some(value) = self.settings.get(key) {
            Ok(*value)
        } else {
            bail!("Unknown setting: {}", key)
        }
    }

    pub fn set(&mut self, key: &str, value: bool) -> Result<()> {
        match key {
            LOGGING_ENABLED => {
                self.settings.insert(key.to_string(), value);
                Ok(())
            }
            _ => bail!("Unknown setting: {}", key),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let mut settings: HashMap<String, bool> = HashMap::new();
        settings.insert(LOGGING_ENABLED.to_string(), false);
        Self { settings }
    }
}

impl SaveData for Settings {
    fn relative_path() -> std::path::PathBuf {
        crate::DATA_DIR.join("config").join("settings.ron")
    }
}
