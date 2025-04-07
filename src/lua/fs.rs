use mlua::{Function, Table, UserData};

use crate::event::Event;

use super::{
    backend::Backend,
    constants::{BACKEND, FS_LISTENERS},
};

pub struct Fs {}

impl UserData for Fs {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("monitor", |ctx, (path, cb): (String, Function)| {
            let table: Table = ctx.named_registry_value(FS_LISTENERS)?;
            table.set(table.raw_len() + 1, cb)?;

            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::FSMonitor(path)).unwrap();
            Ok(())
        })
    }
}
