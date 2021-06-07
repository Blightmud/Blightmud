use mlua::{Lua, UserData, UserDataMethods};

use super::{
    backend::Backend,
    blight::Blight,
    constants::{BACKEND, TIMED_CALLBACK_TABLE, TIMED_CALLBACK_TABLE_CORE, TIMED_NEXT_ID},
};
use crate::event::Event;
use chrono::Duration;
use std::error::Error;
use std::sync::Arc;

pub struct Timer {}

impl Timer {
    pub fn new() -> Self {
        Self {}
    }
}

fn is_core_mode(lua: &Lua) -> Result<bool, mlua::Error> {
    let blight: Blight = lua.globals().get("blight")?;
    Ok(blight.core_mode)
}

fn user_mode_only(lua: &Lua) -> Result<(), mlua::Error> {
    if is_core_mode(lua)? {
        let boxed_error =
            Box::<dyn Error + Send + Sync>::from("this method is not supported in core mode");
        return Err(mlua::Error::ExternalError(Arc::from(boxed_error)));
    }
    Ok(())
}

impl UserData for Timer {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function(
            "add",
            |lua, (duration, count, callback): (f32, u32, mlua::Function)| {
                let duration = Duration::milliseconds((duration * 1000.0) as i64);
                let count = if count > 0 { Some(count) } else { None };
                let core_mode = is_core_mode(lua)?;
                let cb_table_name = if core_mode {
                    TIMED_CALLBACK_TABLE_CORE
                } else {
                    TIMED_CALLBACK_TABLE
                };
                let cb_table: mlua::Table = lua.named_registry_value(cb_table_name)?;
                let backend: Backend = lua.named_registry_value(BACKEND)?;
                let lua_id: mlua::Integer = lua.named_registry_value(TIMED_NEXT_ID)?;
                let id = lua_id as u32;
                cb_table.raw_set(id, callback)?;
                backend
                    .writer
                    .send(Event::AddTimedEvent(duration, count, id, core_mode))
                    .unwrap();
                lua.set_named_registry_value(TIMED_NEXT_ID, id + 1)?;
                Ok(id)
            },
        );
        methods.add_function("get_ids", |ctx, ()| {
            user_mode_only(ctx)?;
            let timer_table: mlua::Table = ctx.named_registry_value(TIMED_CALLBACK_TABLE)?;
            let mut keys: Vec<mlua::Integer> = vec![];
            for pair in timer_table.pairs::<mlua::Integer, mlua::Value>() {
                keys.push(pair?.0);
            }
            Ok(keys)
        });
        methods.add_function("clear", |ctx, ()| {
            user_mode_only(ctx)?;
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            ctx.set_named_registry_value(TIMED_CALLBACK_TABLE, ctx.create_table()?)?;
            backend.writer.send(Event::ClearTimers).unwrap();
            Ok(())
        });
        methods.add_function("remove", |ctx, timer_idx: u32| {
            user_mode_only(ctx)?;
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            let timer_table: mlua::Table = ctx.named_registry_value(TIMED_CALLBACK_TABLE)?;
            timer_table.raw_set(timer_idx, mlua::Nil)?;
            backend.writer.send(Event::RemoveTimer(timer_idx)).unwrap();
            Ok(())
        });
    }
}

#[cfg(test)]
mod test_timer {
    use super::{Backend, Blight, Timer};
    use crate::{
        event::Event,
        lua::constants::{BACKEND, TIMED_CALLBACK_TABLE, TIMED_CALLBACK_TABLE_CORE, TIMED_NEXT_ID},
    };
    use chrono::Duration;
    use mlua::Lua;
    use std::sync::mpsc::{channel, Receiver, Sender};

    #[test]
    fn test_add_timer_core() {
        let lua = Lua::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer.clone());
        let mut blight = Blight::new(writer.clone());
        let timer = Timer::new();
        blight.core_mode(true);

        lua.set_named_registry_value(BACKEND, backend).unwrap();
        lua.set_named_registry_value(TIMED_CALLBACK_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(TIMED_CALLBACK_TABLE_CORE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(TIMED_NEXT_ID, 1).unwrap();
        lua.globals().set("blight", blight).unwrap();
        lua.globals().set("timer", timer).unwrap();

        let add_timer_result: u32 = lua
            .load("return timer.add(1, 2, function () end)")
            .call(())
            .unwrap();
        assert_eq!(add_timer_result, 1);
        assert_eq!(
            reader.recv(),
            Ok(Event::AddTimedEvent(
                Duration::milliseconds(1000),
                Some(2),
                1,
                true,
            ))
        );
        let core_table: mlua::Table = lua.named_registry_value(TIMED_CALLBACK_TABLE_CORE).unwrap();
        assert!(matches!(
            core_table.raw_get(1).unwrap(),
            mlua::Value::Function(_)
        ));
        let new_id: mlua::Integer = lua.named_registry_value(TIMED_NEXT_ID).unwrap();
        assert_eq!(new_id, 2);
    }

    #[test]
    fn test_add_timer_not_core_get_ids() {
        let lua = Lua::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer.clone());
        let mut blight = Blight::new(writer.clone());
        let timer = Timer::new();
        blight.core_mode(false);

        lua.set_named_registry_value(BACKEND, backend).unwrap();
        lua.set_named_registry_value(TIMED_CALLBACK_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(TIMED_CALLBACK_TABLE_CORE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(TIMED_NEXT_ID, 5).unwrap();
        lua.globals().set("blight", blight).unwrap();
        lua.globals().set("timer", timer).unwrap();
        let add_timer_result: u32 = lua
            .load("return timer.add(3, 0, function () end)")
            .call(())
            .unwrap();
        assert_eq!(add_timer_result, 5);
        assert_eq!(
            reader.recv(),
            Ok(Event::AddTimedEvent(
                Duration::milliseconds(3000),
                None,
                5,
                false,
            ))
        );
        let table: mlua::Table = lua.named_registry_value(TIMED_CALLBACK_TABLE).unwrap();
        assert!(matches!(
            table.raw_get(5).unwrap(),
            mlua::Value::Function(_)
        ));
        let new_id: mlua::Integer = lua.named_registry_value(TIMED_NEXT_ID).unwrap();
        assert_eq!(new_id, 6);

        let ids: Vec<u32> = lua.load("return timer.get_ids()").call(()).unwrap();
        assert_eq!(ids, vec![5]);
    }

    #[test]
    fn test_clear_timers() {
        let lua = Lua::new();
        let (writer, _reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer.clone());
        let mut blight = Blight::new(writer.clone());
        let timer = Timer::new();
        blight.core_mode(false);

        lua.set_named_registry_value(BACKEND, backend).unwrap();
        lua.set_named_registry_value(TIMED_CALLBACK_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(TIMED_CALLBACK_TABLE_CORE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(TIMED_NEXT_ID, 1).unwrap();
        lua.globals().set("blight", blight).unwrap();
        lua.globals().set("timer", timer).unwrap();
        let add_timer_result: u32 = lua
            .load("return timer.add(1, 2, function () end)")
            .call(())
            .unwrap();
        assert_eq!(add_timer_result, 1);
        let add_timer_result: u32 = lua
            .load("return timer.add(3, 4, function () end)")
            .call(())
            .unwrap();
        assert_eq!(add_timer_result, 2);
        lua.load("timer.clear()").exec().unwrap();
        let ids: Vec<u32> = lua.load("return timer.get_ids()").call(()).unwrap();
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn test_remove_timer() {
        let lua = Lua::new();
        let (writer, _reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer.clone());
        let mut blight = Blight::new(writer.clone());
        let timer = Timer::new();
        blight.core_mode(false);

        lua.set_named_registry_value(BACKEND, backend).unwrap();
        lua.set_named_registry_value(TIMED_CALLBACK_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(TIMED_CALLBACK_TABLE_CORE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(TIMED_NEXT_ID, 1).unwrap();
        lua.globals().set("blight", blight).unwrap();
        lua.globals().set("timer", timer).unwrap();
        let add_timer_result: u32 = lua
            .load("return timer.add(1, 2, function () end)")
            .call(())
            .unwrap();
        assert_eq!(add_timer_result, 1);
        let add_timer_result: u32 = lua
            .load("return timer.add(3, 4, function () end)")
            .call(())
            .unwrap();
        assert_eq!(add_timer_result, 2);
        lua.load("timer.remove(1)").exec().unwrap();
        let ids: Vec<u32> = lua.load("return timer.get_ids()").call(()).unwrap();
        assert_eq!(ids, vec![2]);
    }
}
