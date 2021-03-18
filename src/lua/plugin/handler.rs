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
        methods.add_function("add", |ctx, url: String| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            let writer = backend.writer;
            add_plugin(writer, &url);
            Ok(())
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
        methods.add_function("update", |ctx, name: String| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            let writer = backend.writer;
            update_plugin(writer, &name);
            Ok(())
        });
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
