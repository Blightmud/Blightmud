use crate::io::SaveData;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default, Serialize, Deserialize)]
pub struct StoreData {
    pub id: String,
    pub data: BTreeMap<String, BTreeMap<String, String>>,
}

impl SaveData for StoreData {
    fn relative_path() -> PathBuf {
        PathBuf::from("store/data.ron")
    }
}
