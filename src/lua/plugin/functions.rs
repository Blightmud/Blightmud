use std::{fs, path::PathBuf, sync::mpsc::Sender};

use anyhow::{bail, Result};
use git2::{
    build::{CloneLocal, RepoBuilder},
    Repository,
};

use crate::event::Event;

fn get_plugin_dir() -> PathBuf {
    let plugin_dir = crate::DATA_DIR.join("plugins");
    fs::create_dir_all(&plugin_dir).ok();
    plugin_dir
}

pub fn add_plugin(url: &str) -> Result<String> {
    if let Some(name) = url.split('/').last() {
        let dest = get_plugin_dir().join(name);
        let mut rbuilder = RepoBuilder::new();
        rbuilder.clone_local(CloneLocal::Auto);
        if let Err(err) = rbuilder.clone(&url, &dest) {
            match err.code() {
                git2::ErrorCode::Exists => bail!("Plugin already exists".to_string()),
                _ => bail!(err.to_string()),
            }
        } else {
            Ok(name.to_string())
        }
    } else {
        bail!(format!("Invalid plugin repository: {}", url))
    }
}

pub fn load_plugin(name: &str, writer: &Sender<Event>) -> Result<()> {
    let path = get_plugin_dir().join(name).join("main.lua");
    if !path.exists() {
        bail!("Plugin '{}' doesn't contain a 'main.lua' file", name);
    } else if let Some(path_name) = path.to_str() {
        writer
            .send(Event::LoadScript(path_name.to_string()))
            .unwrap();
    } else {
        bail!("Invalid plugin path to main.lua");
    }
    Ok(())
}

pub fn remove_plugin(name: &str) -> Result<()> {
    if !name.contains("..") {
        let path = get_plugin_dir().join(name);
        if path.exists() {
            fs::remove_dir_all(path).expect("Plugin doesn't exist");
        }
        Ok(())
    } else {
        bail!("Invalid plugin name");
    }
}

pub fn get_plugins() -> Vec<String> {
    let mut plugins = vec![];
    if let Ok(paths) = fs::read_dir(get_plugin_dir()) {
        for path in paths {
            if let Ok(path) = path {
                if path.path().is_dir() {
                    if let Some(name) = path.file_name().to_str() {
                        plugins.push(name.to_string());
                    }
                }
            }
        }
    }
    plugins
}

pub fn update_plugin(name: &str) -> Result<()> {
    let path = get_plugin_dir().join(name);
    if path.is_dir() {
        let repo = Repository::discover(path)?;
        let mut origin_remote = repo.find_remote("origin")?;
        origin_remote.fetch(&["main"], None, None)?;

        let oid = repo.refname_to_id("refs/remotes/origin/main")?;
        let object = repo.find_object(oid, None)?;
        repo.reset(&object, git2::ResetType::Hard, None)?;
    } else {
        bail!("Invalid plugin name");
    }
    Ok(())
}
