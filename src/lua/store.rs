use crate::io::SaveData;
use rlua::{AnyUserData, Result, UserData, UserDataMethods};
use std::{collections::HashMap, path::PathBuf};

impl SaveData for HashMap<String, String> {
    fn relative_path() -> PathBuf {
        PathBuf::from("store/data.ron")
    }
}

#[derive(Clone)]
pub struct Store {
    pub memory_storage: HashMap<String, String>,
}

impl Store {
    pub const LUA_GLOBAL_NAME: &'static str = "store";

    pub fn new() -> Self {
        Self {
            memory_storage: HashMap::new(),
        }
    }
}

impl UserData for Store {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("session_write", |ctx, (key, val): (String, String)| {
            let store_aud: AnyUserData = ctx.globals().get(Store::LUA_GLOBAL_NAME)?;
            let mut store = store_aud.borrow_mut::<Store>()?;
            store.memory_storage.insert(key, val);
            Ok(())
        });
        methods.add_function(
            "session_read",
            |ctx, key: String| -> Result<Option<String>> {
                let store_aud: AnyUserData = ctx.globals().get(Store::LUA_GLOBAL_NAME)?;
                let store = store_aud.borrow::<Store>()?;
                match store.memory_storage.get(key.as_str()) {
                    Some(val) => Ok(Some(val.to_string())),
                    _ => Ok(None),
                }
            },
        );
        methods.add_function("disk_write", |_ctx, (key, val): (String, String)| {
            let mut persistent_data = HashMap::load();
            persistent_data.insert(key, val);
            persistent_data.save();
            Ok(())
        });
        methods.add_function("disk_read", |_ctx, key: String| -> Result<Option<String>> {
            let persistent_data: HashMap<String, String> = HashMap::load();
            match persistent_data.get(key.as_str()) {
                Some(val) => Ok(Some(val.to_string())),
                _ => Ok(None),
            }
        });
    }
}

#[cfg(test)]
mod test_store {
    use super::Store;
    use rlua::Lua;

    #[test]
    fn test_memory_storage() {
        let lua = Lua::new();
        let store = Store::new();
        lua.context(|ctx| {
            ctx.globals().set(Store::LUA_GLOBAL_NAME, store).unwrap();

            ctx.load("store.session_write(\"abc\",\"def\")")
                .exec()
                .unwrap();
            let value: String = ctx
                .load("return store.session_read(\"abc\")")
                .call(())
                .unwrap();
            assert_eq!("def", value);
        });
    }
}
