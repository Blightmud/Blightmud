use mlua::{UserData, UserDataMethods};

use super::{backend::Backend, constants::BACKEND};
use crate::event::Event;

pub struct Log {}

impl Log {
    pub fn new() -> Self {
        Self {}
    }
}

impl UserData for Log {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("start", |ctx, name: String| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend
                .writer
                .send(Event::StartLogging(name, true))
                .unwrap();
            Ok(())
        });
        methods.add_function("stop", |ctx, _: ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::StopLogging).unwrap();
            Ok(())
        });
    }
}

#[cfg(test)]
mod test_log {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use mlua::Lua;

    use crate::{
        event::Event,
        lua::{backend::Backend, constants::BACKEND},
    };

    use super::Log;

    fn assert_event(lua_code: &str, event: Event) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let log = Log::new();
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.set_named_registry_value(BACKEND, backend).unwrap();
            ctx.globals().set("log", log).unwrap();
            ctx.load(lua_code).exec().unwrap();
        });

        assert_eq!(reader.recv(), Ok(event));
    }

    #[test]
    fn test_start() {
        assert_event(
            "log.start(\"some_name\")",
            Event::StartLogging("some_name".to_string(), true),
        );
    }

    #[test]
    fn test_stop() {
        assert_event("log.stop()", Event::StopLogging);
    }
}
