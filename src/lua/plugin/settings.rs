use std::collections::HashSet;

use crate::io::SaveData;

pub type AutoLoadPlugins = HashSet<String>;

impl SaveData for AutoLoadPlugins {
    fn relative_path() -> std::path::PathBuf {
        crate::DATA_DIR.join("autoload_plugins.ron")
    }

    fn is_pretty() -> bool {
        true
    }
}
