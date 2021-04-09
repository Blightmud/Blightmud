use super::{constants::*, ui_event::UiEvent};
use crate::event::Event;
use crate::{model::Line, PROJECT_NAME, VERSION};
use log::debug;
use rlua::{
    AnyUserData, Function, Result as LuaResult, Table, UserData, UserDataMethods, Variadic,
};
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Blight {
    main_writer: Sender<Event>,
    output_lines: Vec<Line>,
    ui_events: Vec<UiEvent>,
    pub screen_dimensions: (u16, u16),
    pub core_mode: bool,
}

impl Blight {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            output_lines: vec![],
            ui_events: vec![],
            screen_dimensions: (0, 0),
            core_mode: false,
        }
    }

    pub fn core_mode(&mut self, mode: bool) {
        self.core_mode = mode;
    }

    pub fn get_output_lines(&mut self) -> Vec<Line> {
        let return_lines = self.output_lines.clone();
        self.output_lines.clear();
        return_lines
    }

    pub fn get_ui_events(&mut self) -> Vec<UiEvent> {
        let events = self.ui_events.clone();
        self.ui_events.clear();
        events
    }
}

impl UserData for Blight {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("output", |ctx, strings: Variadic<String>| {
            let this_aux = ctx.globals().get::<_, AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            this.output_lines.push(Line::from(strings.join(" ")));
            Ok(())
        });
        methods.add_function("terminal_dimensions", |ctx, _: ()| {
            let this_aux = ctx.globals().get::<_, AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            Ok(this.screen_dimensions)
        });
        methods.add_function("bind", |ctx, (cmd, callback): (String, rlua::Function)| {
            let bind_table: rlua::Table = ctx.globals().get(COMMAND_BINDING_TABLE)?;
            bind_table.set(cmd.to_lowercase(), callback)?;
            Ok(())
        });
        methods.add_function("unbind", |ctx, cmd: String| {
            let bind_table: rlua::Table = ctx.globals().get(COMMAND_BINDING_TABLE)?;
            bind_table.set(cmd, rlua::Nil)?;
            Ok(())
        });
        methods.add_function("ui", |ctx, cmd: String| -> rlua::Result<()> {
            let this_aux = ctx.globals().get::<_, AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            let event: UiEvent = UiEvent::from(cmd.as_str());
            if let UiEvent::Unknown(cmd) = event {
                this.main_writer
                    .send(Event::Error(format!("Invalid ui command: {}", cmd)))
                    .unwrap();
            } else {
                this.ui_events.push(event);
            }
            Ok(())
        });
        methods.add_function("debug", |_, strings: Variadic<String>| {
            debug!("{}", strings.join(" "));
            Ok(())
        });
        methods.add_function("is_core_mode", |ctx, ()| {
            let this_aux = ctx.globals().get::<_, AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            Ok(this.core_mode)
        });
        methods.add_function("status_height", |ctx, height: u16| {
            let this_aux = ctx.globals().get::<_, AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::StatusAreaHeight(height))
                .unwrap();
            Ok(())
        });
        methods.add_function("status_line", |ctx, (index, line): (usize, String)| {
            let this_aux = ctx.globals().get::<_, AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::StatusLine(index, line))
                .unwrap();
            Ok(())
        });
        methods.add_function("version", |_, _: ()| -> LuaResult<(&str, &str)> {
            Ok((PROJECT_NAME, VERSION))
        });
        methods.add_function("config_dir", |_, ()| -> rlua::Result<String> {
            Ok(crate::CONFIG_DIR.to_string_lossy().to_string())
        });
        methods.add_function("data_dir", |_, ()| -> rlua::Result<String> {
            Ok(crate::DATA_DIR.to_string_lossy().to_string())
        });
        methods.add_function("on_quit", |ctx, func: Function| -> rlua::Result<()> {
            let table: Table = ctx.named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)?;
            table.set(table.raw_len() + 1, func)?;
            Ok(())
        });
    }
}

#[cfg(test)]
mod test_blight {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use rlua::{AnyUserData, Lua};

    use crate::event::Event;

    use super::Blight;
    use crate::lua::constants::BLIGHT_ON_QUIT_LISTENER_TABLE;
    use crate::{PROJECT_NAME, VERSION};

    fn get_lua_state() -> (Lua, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let blight = Blight::new(writer);
        let lua = Lua::new();
        lua.context(|ctx| -> rlua::Result<()> {
            ctx.globals().set("blight", blight)?;
            ctx.set_named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE, ctx.create_table()?)?;
            Ok(())
        })
        .unwrap();
        (lua, reader)
    }

    #[test]
    fn test_config_dir() {
        let (lua, _reader) = get_lua_state();
        assert!(lua
            .context(|ctx| -> String { ctx.load("return blight.config_dir()").call(()).unwrap() })
            .ends_with(".run/test/config"));
    }

    #[test]
    fn test_data_dir() {
        let (lua, _reader) = get_lua_state();
        assert!(lua
            .context(|ctx| -> String { ctx.load("return blight.data_dir()").call(()).unwrap() })
            .ends_with(".run/test/data"));
    }

    #[test]
    fn test_version() {
        let (lua, _reader) = get_lua_state();
        assert_eq!(
            lua.context(|ctx| -> (String, String) {
                ctx.load("return blight.version()").call(()).unwrap()
            }),
            (PROJECT_NAME.to_string(), VERSION.to_string())
        );
    }

    #[test]
    fn confirm_on_quite_register() {
        let (lua, _reader) = get_lua_state();
        lua.context(|ctx| {
            let table: rlua::Table = ctx
                .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
                .unwrap();
            assert_eq!(table.raw_len(), 0);
            ctx.load("blight.on_quit(function () end)").exec().unwrap();
            let table: rlua::Table = ctx
                .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
                .unwrap();
            assert_eq!(table.raw_len(), 1);
        });
    }

    #[test]
    fn on_quit_function() {
        let (lua, _reader) = get_lua_state();
        lua.context(|ctx| -> rlua::Result<()> {
            ctx.load("blight.on_quit(function () blight.output(\"on_quit\") end)")
                .exec()
                .unwrap();
            let table: rlua::Table = ctx
                .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
                .unwrap();
            for pair in table.pairs::<rlua::Value, rlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>(()).unwrap();
            }
            let blight_aux = ctx.globals().get::<_, AnyUserData>("blight")?;
            let mut blight = blight_aux.borrow_mut::<Blight>()?;
            let lines = blight.get_output_lines();
            let mut it = lines.iter();
            assert_eq!(it.next().unwrap(), &crate::model::Line::from("on_quit"));
            Ok(())
        })
        .unwrap();
    }
}
