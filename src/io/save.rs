use crate::DATA_DIR;

use anyhow::{bail, Result};
use log::error;
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

    fn load() -> Self {
        let read_data = || -> Result<Self> {
            let path = Self::path()?;
            if path.exists() {
                let file = fs::File::open(&path)?;
                let obj = ron::de::from_reader(&file)?;
                Ok(obj)
            } else {
                bail!("Bad path")
            }
        };

        match read_data() {
            Ok(obj) => obj,
            Err(err) => {
                error!("Write data error: {}", err.to_string());
                Self::default()
            }
        }
    }

    fn save(&self) {
        let write_data = || -> Result<()> {
            let contents = ron::ser::to_string(&self)?;
            fs::write(Self::path()?, contents)?;
            Ok(())
        };

        if let Err(err) = write_data() {
            error!("Write data error: {}", err.to_string());
        }
    }
}
