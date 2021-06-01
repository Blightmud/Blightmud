use crate::DATA_DIR;

use anyhow::Result;
use log::error;
use serde::{de::DeserializeOwned, Serialize};

use std::fs;
use std::path::PathBuf;

pub trait SaveData: DeserializeOwned + Serialize + Default {
    fn relative_path() -> PathBuf;

    fn is_pretty() -> bool {
        false
    }

    fn on_load(&mut self) {}

    fn path() -> Result<PathBuf> {
        let path = DATA_DIR.join(Self::relative_path());

        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        Ok(path)
    }

    fn try_load() -> Result<Self> {
        let path = Self::path()?;
        if path.exists() {
            let file = fs::File::open(&path)?;
            let mut obj: Self = ron::de::from_reader(&file)?;
            obj.on_load();
            Ok(obj)
        } else {
            Ok(Self::default())
        }
    }

    fn load() -> Self {
        Self::try_load()
            .map_err(|err| error!("Load data error: {}", err))
            .unwrap_or_default()
    }

    fn save(&self) {
        let write_data = || -> Result<()> {
            let contents = if Self::is_pretty() {
                ron::ser::to_string_pretty(&self, Default::default())?
            } else {
                ron::ser::to_string(&self)?
            };
            fs::write(Self::path()?, contents)?;
            Ok(())
        };

        if let Err(err) = write_data() {
            error!("Write data error: {}", err.to_string());
        }
    }
}
