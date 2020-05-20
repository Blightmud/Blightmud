use anyhow::{format_err, Result};
use serde::{de::DeserializeOwned, Serialize};

use std::fs;
use std::path::PathBuf;

pub trait SaveData: DeserializeOwned + Serialize + Default {
    fn relative_path() -> PathBuf;

    fn path() -> Result<PathBuf> {
        let mut data_dir =
            dirs::data_dir().ok_or_else(|| format_err!("Could not get data directory"))?;
        data_dir.push("blightmud");

        let save_path = data_dir.join(Self::relative_path());

        if let Some(save_dir) = save_path.parent() {
            std::fs::create_dir_all(save_dir)?;
        }

        Ok(save_path)
    }

    fn load() -> Result<Self> {
        let save_path = Self::path()?;

        if save_path.exists() {
            let file = fs::File::open(&save_path)?;

            Ok(ron::de::from_reader(&file)?)
        } else {
            Ok(Self::default())
        }
    }

    fn save(&self) -> Result<()> {
        let contents = ron::ser::to_string(&self)?;

        fs::write(Self::path()?, contents)?;

        Ok(())
    }
}
