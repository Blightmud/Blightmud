use std::{fs, path::PathBuf};

use crate::lua::LuaScript;

fn get_plugin_dir() -> PathBuf {
    let plugin_dir = crate::CONFIG_DIR.clone().join("plugins");
    fs::create_dir_all(&plugin_dir).ok();
    plugin_dir
}

pub fn _add_plugin(_lua: &mut LuaScript, _path: &str) {}

pub fn _load_plugins(_lua: &mut LuaScript) {
    if let Ok(paths) = fs::read_dir(get_plugin_dir()) {
        for path in paths {
            if let Ok(path) = path {
                if path.path().is_dir() {}
            }
        }
    }
}

pub fn _update_plugins() {}
