use std::{fs, path::PathBuf, sync::mpsc::Sender};

use anyhow::{bail, Result};
use git2::{
    build::{CloneLocal, RepoBuilder},
    Repository,
};

use crate::event::Event;

pub fn get_plugin_dir() -> PathBuf {
    let plugin_dir = crate::DATA_DIR.join("plugins");
    fs::create_dir_all(&plugin_dir).ok();
    plugin_dir
}

pub fn add_plugin(main_writer: Sender<Event>, url: &str, with_submodules: bool) {
    let url = url.to_string();
    std::thread::spawn(move || {
        if let Some(name) = url.split('/').last() {
            let dest = get_plugin_dir().join(name);
            let mut rbuilder = RepoBuilder::new();
            rbuilder.clone_local(CloneLocal::Auto);
            main_writer
                .send(Event::Info(format!(
                    "Downloading plugin: {} from {}",
                    name, url
                )))
                .unwrap();
            if let Err(err) = rbuilder.clone(&url, &dest) {
                match err.code() {
                    git2::ErrorCode::Exists => main_writer
                        .send(Event::Error("Plugin already exists".to_string()))
                        .unwrap(),
                    _ => main_writer.send(Event::Error(err.to_string())).unwrap(),
                }
            } else {
                main_writer
                    .send(Event::Info(format!("Downloaded plugin: {}", name)))
                    .unwrap();
                if with_submodules {
                    main_writer
                        .send(Event::Info(format!("Getting the submodules for {}.", name)))
                        .unwrap();
                    if let Ok(repo) = Repository::discover(&dest) {
                        match update_submodules(repo, true) {
                            Ok(()) => main_writer
                                .send(Event::Info("Plugin retrieval succeeded.".to_string()))
                                .unwrap(),
                            Err(e) => main_writer
                                .send(Event::Error(format!(
                                    "problems updating the submodules for this plugin: {}",
                                    e
                                )))
                                .unwrap(),
                        }
                    } else {
                        main_writer
                            .send(Event::Error("Problem opening repository.".to_string()))
                            .unwrap();
                    }
                }
            }
        } else {
            main_writer
                .send(Event::Error(format!("Invalid plugin repository: {}", url)))
                .unwrap();
        }
    });
}

pub fn update_plugin(main_writer: Sender<Event>, name: &str) {
    let name = name.to_string();
    std::thread::spawn(move || {
        let updater = || -> Result<()> {
            let path = get_plugin_dir().join(&name);
            if path.is_dir() {
                let repo = Repository::discover(path)?;
                let mut origin_remote = repo.find_remote("origin")?;
                origin_remote.fetch(&["master", "main"], None, None)?;

                let oid = if let Ok(oid) = repo.refname_to_id("refs/remotes/origin/master") {
                    oid
                } else {
                    repo.refname_to_id("refs/remotes/origin/main")?
                };

                let object = repo.find_object(oid, None)?;
                repo.reset(&object, git2::ResetType::Hard, None)?;
                Ok(())
            } else {
                bail!("Invalid plugin name");
            }
        };
        main_writer
            .send(Event::Info(format!("Updating plugin: {}", &name)))
            .unwrap();
        if let Err(err) = updater() {
            main_writer
                .send(Event::Error(format!(
                    "Plugin update failed: {}",
                    err.to_string()
                )))
                .unwrap();
        } else {
            main_writer
                .send(Event::Info(format!("Updated plugin: {}", &name)))
                .unwrap();
            // Given that the reset hard cleaned out the repo, we need to remake it.
            if let Ok(repo) = Repository::discover(get_plugin_dir().join(&name)) {
                // Now we need to account for the submodules
                main_writer
                    .send(Event::Info("Updating the plugin's submodules.".to_string()))
                    .unwrap();
                match update_submodules(repo, false) {
                    Ok(()) => main_writer
                        .send(Event::Info(
                            "The update of the submodules was successful.".to_string(),
                        ))
                        .unwrap(),
                    Err(e) => main_writer
                        .send(Event::Error(format!("Error updating submodules; {}", e)))
                        .unwrap(),
                }
            } else {
                main_writer
                    .send(Event::Error("Failed to open the repository.".to_string()))
                    .unwrap();
            }
        }
    });
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
        for path in paths.flatten() {
            if path.path().is_dir() {
                if let Some(name) = path.file_name().to_str() {
                    plugins.push(name.to_string());
                }
            }
        }
    }
    plugins
}
fn update_submodules(repository: Repository, should_initialize: bool) -> Result<()> {
    match repository.submodules() {
        Ok(sms) => {
            for mut sm in sms {
                if let Err(e) = sm.update(should_initialize, None) {
                    match e.message() {
                        "submodule is not initialized" => {
                            continue;
                        }
                        _ => {
                            bail!("Problem updating the submodule: {}.", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            bail!("Error getting access to the submodules: {}", e);
        }
    }
    Ok(())
}
