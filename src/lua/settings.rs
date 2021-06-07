use super::{backend::Backend, constants::BACKEND};
use crate::{event::Event, io::SaveData, model};
use mlua::{Error, Result, Table, UserData, UserDataMethods};

pub struct Settings {}

impl Settings {
    pub const LUA_GLOBAL_NAME: &'static str = "settings";

    pub fn new() -> Self {
        Self {}
    }
}

impl UserData for Settings {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("list", |ctx, _: ()| -> Result<Table<'lua>> {
            let settings = model::Settings::try_load().map_err(Error::external)?;
            let result = ctx.create_table()?;
            model::SETTINGS.iter().try_for_each(|key| {
                result.set(key.to_string(), settings.get(key).map_err(Error::external)?)
            })?;
            Ok(result)
        });
        methods.add_function("get", |_ctx, key: String| -> Result<bool> {
            let settings = model::Settings::try_load().map_err(Error::external)?;
            settings.get(key.as_str()).map_err(Error::external)
        });
        methods.add_function("set", |ctx, (key, val): (String, bool)| {
            let mut settings = model::Settings::try_load().map_err(Error::external)?;
            settings.set(key.as_str(), val).map_err(Error::external)?;
            settings.save();
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend
                .writer
                .send(Event::SettingChanged(key, val))
                .map_err(Error::external)?;
            Ok(())
        });
    }
}

#[cfg(test)]
mod test_settings {
    use super::Settings;
    use crate::model;
    use mlua::Lua;

    #[test]
    fn test_list_settings() {
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.globals()
                .set(Settings::LUA_GLOBAL_NAME, Settings::new())
                .unwrap();

            let settings_table: mlua::Table = ctx.load("return settings.list()").call(()).unwrap();

            assert!(matches!(
                settings_table.raw_get(model::MOUSE_ENABLED).unwrap(),
                mlua::Value::Boolean(_),
            ));
        });
    }

    #[test]
    fn test_get_settings() {
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.globals()
                .set(Settings::LUA_GLOBAL_NAME, Settings::new())
                .unwrap();

            let value: mlua::Value = ctx
                .load("return settings.get(\"mouse_enabled\")")
                .call(())
                .unwrap();

            assert!(matches!(value, mlua::Value::Boolean(_)));
        });
    }
}
