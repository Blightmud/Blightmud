use mlua::{UserData, UserDataMethods};

use crate::event::Event;

use super::{
    backend::Backend,
    constants::{BACKEND, SCRIPT_RESET_LISTENERS},
};

#[derive(Clone)]
pub struct Script {}

impl UserData for Script {
    fn add_methods<T: UserDataMethods<Self>>(methods: &mut T) {
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
        methods.add_function("on_reset", |ctx, cb: mlua::Function| {
            let listeners: mlua::Table = ctx.named_registry_value(SCRIPT_RESET_LISTENERS)?;
            listeners.set(listeners.raw_len() + 1, cb)?;
            Ok(())
        })
    }
}
