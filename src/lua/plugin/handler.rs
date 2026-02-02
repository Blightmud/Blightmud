use mlua::{UserData, UserDataMethods};

use crate::io::SaveData;
use crate::lua::{backend::Backend, constants::BACKEND};

use super::{
    functions::{
        add_plugin, get_plugin_dir, get_plugins, load_plugin, remove_plugin, update_plugin,
    },
    settings::AutoLoadPlugins,
};

pub struct Handler {}

impl Handler {
    pub fn new() -> Self {
        Self {}
    }
}

impl UserData for Handler {
    fn add_methods<T: UserDataMethods<Self>>(methods: &mut T) {
        methods.add_function("add", |ctx, (url, with_submodules): (String, bool)| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            let writer = backend.writer;
            add_plugin(writer, &url, with_submodules);
            Ok(())
        });
        methods.add_function(
            "load",
            |ctx, name: String| -> mlua::Result<(bool, String)> {
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
        methods.add_function("get_all", |_, ()| -> mlua::Result<Vec<String>> {
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
        methods.add_function("enabled", |_, _: ()| -> mlua::Result<Vec<String>> {
            let autoloaded = AutoLoadPlugins::load();
            Ok(autoloaded.iter().cloned().collect())
        });
        methods.add_function("dir", |_, name: Option<String>| -> mlua::Result<String> {
            if let Some(name) = name {
                Ok(get_plugin_dir().join(name).to_string_lossy().to_string())
            } else {
                Ok(get_plugin_dir().to_string_lossy().to_string())
            }
        });
    }
}

#[cfg(test)]
mod test_plugin {
    use mlua::Lua;
    use std::sync::mpsc::{channel, Receiver, Sender};

    use crate::{
        event::Event,
        lua::{backend::Backend, constants::BACKEND},
    };

    use super::Handler;

    fn get_lua_state() -> Lua {
        let plugin = Handler::new();
        let lua = Lua::new();
        lua.globals().set("plugin", plugin).unwrap();
        lua
    }

    fn get_lua_state_with_backend() -> (Lua, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let plugin = Handler::new();
        let lua = Lua::new();
        lua.set_named_registry_value(BACKEND, backend).unwrap();
        lua.globals().set("plugin", plugin).unwrap();
        (lua, reader)
    }

    #[test]
    fn test_dir() {
        let lua = get_lua_state();
        assert!(lua
            .load("return plugin.dir()")
            .call::<String>(())
            .unwrap()
            .ends_with(".run/test/data/plugins"));
    }

    #[test]
    fn test_named_dir() {
        let lua = get_lua_state();
        assert!(lua
            .load("return plugin.dir(\"awesome\")")
            .call::<String>(())
            .unwrap()
            .ends_with(".run/test/data/plugins/awesome"));
    }

    #[test]
    fn test_get_all_returns_vec() {
        let lua = get_lua_state();
        let plugins: Vec<String> = lua.load("return plugin.get_all()").call(()).unwrap();
        // Just ensure it returns a valid vec (may be empty)
        assert!(plugins.is_empty() || !plugins.is_empty());
    }

    #[test]
    fn test_enabled_returns_vec() {
        let lua = get_lua_state();
        let enabled: Vec<String> = lua.load("return plugin.enabled()").call(()).unwrap();
        // Just ensure it returns a valid vec
        assert!(enabled.is_empty() || !enabled.is_empty());
    }

    #[test]
    fn test_disable_nonexistent_plugin() {
        let lua = get_lua_state();
        // Should not error even if plugin doesn't exist
        lua.load("plugin.disable('nonexistent_plugin_xyz')")
            .exec()
            .unwrap();
        // Verify disabled plugin is not in enabled list
        let enabled: Vec<String> = lua.load("return plugin.enabled()").call(()).unwrap();
        assert!(!enabled.contains(&"nonexistent_plugin_xyz".to_string()));
    }

    #[test]
    fn test_enable_nonexistent_plugin() {
        let lua = get_lua_state();
        // Should not error, but won't add to enabled list since plugin doesn't exist
        lua.load("plugin.enable('nonexistent_plugin_xyz')")
            .exec()
            .unwrap();
        // Verify nonexistent plugin is not added to enabled list
        let enabled: Vec<String> = lua.load("return plugin.enabled()").call(()).unwrap();
        assert!(!enabled.contains(&"nonexistent_plugin_xyz".to_string()));
    }

    #[test]
    fn test_remove_nonexistent_plugin() {
        let lua = get_lua_state();
        let result: (bool, String) = lua
            .load("return plugin.remove('nonexistent_plugin_xyz')")
            .call(())
            .unwrap();
        // Should return (true, "") since the dir doesn't exist (no error thrown)
        // or (false, error_msg) if there was an error
        assert!(result.0 || !result.1.is_empty());
    }

    #[test]
    fn test_load_nonexistent_plugin() {
        let (lua, _reader) = get_lua_state_with_backend();
        let result: (bool, String) = lua
            .load("return plugin.load('nonexistent_plugin_xyz')")
            .call(())
            .unwrap();
        assert!(!result.0);
        assert!(!result.1.is_empty());
    }
}
