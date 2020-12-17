use rlua::{UserData, UserDataMethods};

use crate::io::SaveData;
use crate::lua::{backend::Backend, constants::BACKEND};

use super::{
    functions::{add_plugin, get_plugins, load_plugin, remove_plugin, update_plugin},
    settings::AutoLoadPlugins,
};

pub struct Handler {}

impl Handler {
    pub fn new() -> Self {
        Self {}
    }
}

impl UserData for Handler {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function("add", |_, url: String| -> rlua::Result<(String, String)> {
            match add_plugin(&url) {
                Ok(name) => Ok((name, String::new())),
                Err(err) => Ok(("".to_string(), err.to_string())),
            }
        });
        methods.add_function(
            "load",
            |ctx, name: String| -> rlua::Result<(bool, String)> {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                if let Err(err) = load_plugin(&name, &backend.writer) {
                    Ok((false, err.to_string()))
                } else {
                    Ok((true, String::new()))
                }
            },
        );
        methods.add_function("remove", |_, name: String| {
            let mut auto = AutoLoadPlugins::load();
            auto.remove(&name);
            auto.save();
            if let Err(err) = remove_plugin(&name) {
                Ok((false, err.to_string()))
            } else {
                Ok((true, String::new()))
            }
        });
        methods.add_function("get_all", |_, ()| -> rlua::Result<Vec<String>> {
            Ok(get_plugins())
        });
        methods.add_function(
            "update",
            |_, name: String| -> rlua::Result<(bool, String)> {
                if let Err(err) = update_plugin(&name) {
                    Ok((false, err.to_string()))
                } else {
                    Ok((true, String::new()))
                }
            },
        );
        methods.add_function("enable", |_, name: String| {
            if get_plugins().contains(&name) {
                let mut auto = AutoLoadPlugins::load();
                auto.insert(name);
                auto.save();
            }
            Ok(())
        });
        methods.add_function("disable", |_, name: String| {
            let mut auto = AutoLoadPlugins::load();
            auto.remove(&name);
            auto.save();
            Ok(())
        });
        methods.add_function("enabled", |_, _: ()| -> rlua::Result<Vec<String>> {
            let autoloaded = AutoLoadPlugins::load();
            Ok(autoloaded.iter().cloned().collect())
        });
    }
}
