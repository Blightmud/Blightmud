use crate::io::SaveData;
use std::{collections::BTreeMap, path::PathBuf};

pub type StoreData = BTreeMap<String, BTreeMap<String, String>>;

impl SaveData for StoreData {
    fn relative_path() -> PathBuf {
        PathBuf::from("store/data.ron")
    }
}
