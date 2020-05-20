use crate::DATA_DIR;

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};

use std::fs;
use std::path::PathBuf;

pub trait SaveData: DeserializeOwned + Serialize + Default {
    fn relative_path() -> PathBuf;

    fn path() -> Result<PathBuf> {
        let path = DATA_DIR.join(Self::relative_path());

        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }

        Ok(path)
    }

    fn load() -> Result<Self> {
        let path = Self::path()?;

        if path.exists() {
            let file = fs::File::open(&path)?;

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
