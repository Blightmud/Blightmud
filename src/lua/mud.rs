use rlua::{Function, Table, UserData, UserDataMethods};

use crate::{
    event::Event,
    model::{Connection, Line},
};

use super::{
    backend::Backend,
    constants::{
        BACKEND, CONNECTION_ID, MUD_INPUT_LISTENER_TABLE, MUD_OUTPUT_LISTENER_TABLE,
        ON_CONNECTION_CALLBACK_TABLE, ON_DISCONNECT_CALLBACK_TABLE,
    },
};

pub struct Mud {}

impl Mud {
    pub fn new() -> Self {
        Self {}
    }
}

impl UserData for Mud {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function(
            "add_output_listener",
            |ctx, func: Function| -> rlua::Result<()> {
                let table: Table = ctx.named_registry_value(MUD_OUTPUT_LISTENER_TABLE)?;
                table.set(table.raw_len() + 1, func)?;
                Ok(())
            },
        );
        methods.add_function(
            "add_input_listener",
            |ctx, func: Function| -> rlua::Result<()> {
                let table: Table = ctx.named_registry_value(MUD_INPUT_LISTENER_TABLE)?;
                table.set(table.raw_len() + 1, func)?;
                Ok(())
            },
        );
        methods.add_function("output", |ctx, msg: String| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend
                .writer
                .send(Event::MudOutput(Line::from(msg)))
                .unwrap();
            Ok(())
        });
        methods.add_function("connect", |ctx, (host, port, tls): (String, u16, bool)| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend
                .writer
                .send(Event::Connect(Connection { host, port, tls }))
                .unwrap();
            Ok(())
        });
        methods.add_function("disconnect", |ctx, ()| {
            let conn_id: u16 = ctx.named_registry_value(CONNECTION_ID).unwrap_or_default();
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::Disconnect(conn_id)).unwrap();
            Ok(())
        });
        methods.add_function("reconnect", |ctx, ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::Reconnect).unwrap();
            Ok(())
        });
        methods.add_function(
            "send",
            |ctx, (msg, options): (String, Option<rlua::Table>)| {
                let mut line = Line::from(msg);
                line.flags.bypass_script = true;

                if let Some(table) = options {
                    line.flags.gag = table.get("gag")?;
                    line.flags.skip_log = table.get("skip_log")?;
                }

                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend.writer.send(Event::ServerInput(line)).unwrap();
                Ok(())
            },
        );
        methods.add_function("send_bytes", |ctx, bytes: Vec<u8>| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::ServerSend(bytes)).unwrap();
            Ok(())
        });
        methods.add_function("input", |ctx, line: String| {
            let line = Line::from(line);
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::ServerInput(line)).unwrap();
            Ok(())
        });
        methods.add_function("on_connect", |ctx, callback: rlua::Function| {
            let globals = ctx.globals();
            let table: rlua::Table = globals.get(ON_CONNECTION_CALLBACK_TABLE)?;
            table.raw_set(table.raw_len() + 1, callback)?;
            Ok(())
        });
        methods.add_function("on_disconnect", |ctx, callback: rlua::Function| {
            let globals = ctx.globals();
            let table: rlua::Table = globals.get(ON_DISCONNECT_CALLBACK_TABLE)?;
            table.set(table.raw_len() + 1, callback)?;
            Ok(())
        });
    }
}

#[cfg(test)]
mod test_mud {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use rlua::Lua;

    use crate::{
        event::Event,
        lua::constants::MUD_INPUT_LISTENER_TABLE,
        lua::constants::MUD_OUTPUT_LISTENER_TABLE,
        lua::{backend::Backend, constants::BACKEND},
        model::Connection,
        model::Line,
    };

    use super::{Mud, CONNECTION_ID};

    #[test]
    fn test_output_register() {
        let mud = Mud::new();
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.set_named_registry_value(MUD_OUTPUT_LISTENER_TABLE, ctx.create_table().unwrap())
                .unwrap();
            ctx.globals().set("mud", mud).unwrap();
            ctx.load("mud.add_output_listener(function () end)")
                .exec()
                .unwrap();
            let table: rlua::Table = ctx.named_registry_value(MUD_OUTPUT_LISTENER_TABLE).unwrap();
            assert_eq!(table.raw_len(), 1);
        });
    }

    #[test]
    fn test_input_register() {
        let mud = Mud::new();
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.set_named_registry_value(MUD_INPUT_LISTENER_TABLE, ctx.create_table().unwrap())
                .unwrap();
            ctx.globals().set("mud", mud).unwrap();
            ctx.load("mud.add_input_listener(function () end)")
                .exec()
                .unwrap();
            let table: rlua::Table = ctx.named_registry_value(MUD_INPUT_LISTENER_TABLE).unwrap();
            assert_eq!(table.raw_len(), 1);
        });
    }

    fn assert_event(lua_code: &str, event: Event) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let mud = Mud::new();
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.set_named_registry_value(BACKEND, backend).unwrap();
            ctx.globals().set("mud", mud).unwrap();
            ctx.load(lua_code).exec().unwrap();
        });

        assert_eq!(reader.recv(), Ok(event));
    }

    #[test]
    fn test_connect() {
        assert_event(
            "mud.connect(\"hostname\", 99)",
            Event::Connect(Connection {
                host: "hostname".to_string(),
                port: 99,
                tls: false,
            }),
        );
        assert_event(
            "mud.connect(\"hostname\", 99, false)",
            Event::Connect(Connection {
                host: "hostname".to_string(),
                port: 99,
                tls: false,
            }),
        );
        assert_event(
            "mud.connect(\"hostname\", 99, true)",
            Event::Connect(Connection {
                host: "hostname".to_string(),
                port: 99,
                tls: true,
            }),
        );
    }

    #[test]
    fn test_default_disconnect() {
        assert_event("mud.disconnect()", Event::Disconnect(0));
    }

    #[test]
    fn test_disconnect() {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let mud = Mud::new();
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.set_named_registry_value(BACKEND, backend).unwrap();
            ctx.set_named_registry_value(CONNECTION_ID, 4).unwrap();
            ctx.globals().set("mud", mud).unwrap();
            ctx.load("mud.disconnect()").exec().unwrap();
            assert_eq!(reader.recv().unwrap(), Event::Disconnect(4));
        });
    }

    #[test]
    fn test_send_bytes() {
        assert_event(
            "mud.send_bytes({ 0xff, 0xf1 })",
            Event::ServerSend(vec![0xff, 0xf1]),
        );
    }

    #[test]
    fn test_mud_output_command() {
        let lua_code = r#"
        mud.output("test trigger")
        "#;
        assert_event(&lua_code, Event::MudOutput(Line::from("test trigger")));
    }

    #[test]
    fn test_user_input_command() {
        let lua_code = r#"
        mud.input("test line")
        "#;

        assert_event(&lua_code, Event::ServerInput(Line::from("test line")));
    }
}
