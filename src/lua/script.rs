use mlua::{UserData, UserDataMethods};

use crate::event::Event;

use super::{backend::Backend, constants::BACKEND};

#[derive(Clone)]
pub struct Script {}

impl UserData for Script {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function("load", |ctx, path: String| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::LoadScript(path)).unwrap();
            Ok(())
        });
        methods.add_function("reset", |ctx, ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::ResetScript).unwrap();
            Ok(())
        });
    }
}
