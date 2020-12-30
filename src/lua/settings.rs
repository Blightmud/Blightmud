use super::{backend::Backend, constants::BACKEND};
use crate::{event::Event, io::SaveData, model};
use rlua::{Error, Result, Table, UserData, UserDataMethods};

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
            let settings =
                model::Settings::try_load().map_err(|anyhow_err| Error::external(anyhow_err))?;
            let result = ctx.create_table()?;
            model::SETTINGS.iter().try_for_each(|key| {
                result.set(
                    key.to_string(),
                    settings
                        .get(key)
                        .map_err(|anyhow_err| Error::external(anyhow_err))?,
                )
            })?;
            Ok(result)
        });
        methods.add_function("get", |_ctx, key: String| -> Result<bool> {
            let settings =
                model::Settings::try_load().map_err(|anyhow_err| Error::external(anyhow_err))?;
            settings
                .get(key.as_str())
                .map_err(|anyhow_err| Error::external(anyhow_err))
        });
        methods.add_function("set", |ctx, (key, val): (String, bool)| {
            let mut settings =
                model::Settings::try_load().map_err(|anyhow_err| Error::external(anyhow_err))?;
            settings
                .set(key.as_str(), val)
                .map_err(|anyhow_err| Error::external(anyhow_err))?;
            settings.save();
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend
                .writer
                .send(Event::SettingChanged(key, val))
                .map_err(|send_err| Error::external(send_err))?;
            Ok(())
        });
    }
}

#[cfg(test)]
mod test_settings {
    use super::Settings;
    use crate::model;
    use rlua::Lua;

    #[test]
    fn test_list_settings() {
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.globals()
                .set(Settings::LUA_GLOBAL_NAME, Settings::new())
                .unwrap();

            let settings_table: rlua::Table = ctx.load("return settings.list()").call(()).unwrap();

            assert!(matches!(
                settings_table.raw_get(model::MOUSE_ENABLED).unwrap(),
                rlua::Value::Boolean(_),
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

            let value: rlua::Value = ctx
                .load("return settings.get(\"mouse_enabled\")")
                .call(())
                .unwrap();

            assert!(matches!(value, rlua::Value::Boolean(_)));
        });
    }
}
