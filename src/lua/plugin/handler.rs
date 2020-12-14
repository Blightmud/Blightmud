use rlua::{UserData, UserDataMethods};

use crate::lua::{backend::Backend, constants::BACKEND};

use super::functions::{
    add_plugin, get_plugins, load_plugin, remove_plugin, update_plugin,
};

pub struct Handler {}

impl Handler {
    pub fn new() -> Self {
        Self {}
    }
}

impl UserData for Handler {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function("add", |_, url: String| -> rlua::Result<(bool, String)> {
            if let Err(err) = add_plugin(&url) {
                Ok((false, err.to_string()))
            } else {
                Ok((true, String::new()))
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
    }
}
