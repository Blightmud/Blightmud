use super::{constants::*, regex::Regex, ui_event::UiEvent};
use crate::event::{Event, QuitMethod};
use crate::{model::Line, PROJECT_NAME, VERSION};
use log::debug;
use mlua::{
    AnyUserData, FromLua, Function, Result as LuaResult, Table, UserData, UserDataMethods, Variadic,
};
use std::sync::mpsc::Sender;

#[derive(Clone, FromLua)]
pub struct Blight {
    main_writer: Sender<Event>,
    output_lines: Vec<Line>,
    ui_events: Vec<UiEvent>,
    pub screen_dimensions: (u16, u16),
    pub core_mode: bool,
    pub reader_mode: bool,
    pub _tts_enabled: bool,
}

impl Blight {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            output_lines: vec![],
            ui_events: vec![],
            screen_dimensions: (0, 0),
            core_mode: false,
            reader_mode: false,
            _tts_enabled: false,
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
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("output", |ctx, strings: Variadic<String>| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            this.output_lines.push(Line::from(strings.join(" ")));
            Ok(())
        });
        methods.add_function("terminal_dimensions", |ctx, _: ()| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            Ok(this.screen_dimensions)
        });
        methods.add_function("bind", |ctx, (cmd, callback): (String, mlua::Function)| {
            let bind_table: mlua::Table = ctx.named_registry_value(COMMAND_BINDING_TABLE)?;
            if cmd.to_lowercase().starts_with("alt-") {
                let (_, right) = cmd.split_at(3);
                let mut cmd = "alt".to_string();
                cmd.push_str(right);
                bind_table.set(cmd, callback)?;
            } else {
                bind_table.set(cmd.to_lowercase(), callback)?;
            }
            Ok(())
        });
        methods.add_function("unbind", |ctx, cmd: String| {
            let bind_table: mlua::Table = ctx.named_registry_value(COMMAND_BINDING_TABLE)?;
            bind_table.set(cmd, mlua::Nil)?;
            Ok(())
        });
        methods.add_function("ui", |ctx, cmd: String| -> mlua::Result<()> {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let mut this = this_aux.borrow_mut::<Blight>()?;
            let event: UiEvent = UiEvent::from(cmd.as_str());
            if let UiEvent::Unknown(cmd) = event {
                this.main_writer
                    .send(Event::Error(format!("Invalid ui command: {cmd}")))
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
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            Ok(this.core_mode)
        });
        methods.add_function("is_reader_mode", |ctx, ()| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            Ok(this.reader_mode)
        });
        methods.add_function("status_height", |ctx, requested: Option<u16>| {
            let height: u16 = if let Some(height) = requested {
                let height = height.clamp(0, 5);
                let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
                let this = this_aux.borrow::<Blight>()?;
                this.main_writer
                    .send(Event::StatusAreaHeight(height))
                    .unwrap();
                ctx.set_named_registry_value(STATUS_AREA_HEIGHT, height)?;
                height
            } else {
                ctx.named_registry_value(STATUS_AREA_HEIGHT)?
            };
            Ok(height)
        });
        methods.add_function("status_line", |ctx, (index, line): (usize, String)| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::StatusLine(index, line))
                .unwrap();
            Ok(())
        });
        methods.add_function("version", |_, _: ()| -> LuaResult<(&str, &str)> {
            Ok((PROJECT_NAME, VERSION))
        });
        methods.add_function("config_dir", |_, ()| -> mlua::Result<String> {
            Ok(crate::CONFIG_DIR.to_string_lossy().to_string())
        });
        methods.add_function("data_dir", |_, ()| -> mlua::Result<String> {
            Ok(crate::DATA_DIR.to_string_lossy().to_string())
        });
        methods.add_function("on_quit", |ctx, func: Function| -> mlua::Result<()> {
            let table: Table = ctx.named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)?;
            table.set(table.raw_len() + 1, func)?;
            Ok(())
        });
        methods.add_function(
            "on_complete",
            |ctx, func: mlua::Function| -> mlua::Result<()> {
                let table: Table = ctx.named_registry_value(COMPLETION_CALLBACK_TABLE)?;
                table.set(table.raw_len() + 1, func)?;
                Ok(())
            },
        );
        methods.add_function(
            "on_dimensions_change",
            |ctx, func: Function| -> mlua::Result<()> {
                let table: Table =
                    ctx.named_registry_value(BLIGHT_ON_DIMENSIONS_CHANGE_LISTENER_TABLE)?;
                table.set(table.raw_len() + 1, func)?;
                Ok(())
            },
        );
        methods.add_function("quit", |ctx, ()| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::Quit(QuitMethod::Script))
                .unwrap();
            Ok(())
        });
        methods.add_function("show_help", |ctx, (name, lock_scroll): (String, bool)| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::ShowHelp(name, lock_scroll))
                .unwrap();
            Ok(())
        });
        methods.add_function("find_backward", |ctx, re: Regex| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer
                .send(Event::FindBackward(re.regex))
                .unwrap();
            Ok(())
        });
        methods.add_function("find_forward", |ctx, re: Regex| {
            let this_aux = ctx.globals().get::<AnyUserData>("blight")?;
            let this = this_aux.borrow::<Blight>()?;
            this.main_writer.send(Event::FindForward(re.regex)).unwrap();
            Ok(())
        });
    }
}

#[cfg(test)]
mod test_blight {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use mlua::{AnyUserData, Lua};

    use crate::event::{Event, QuitMethod};
    use crate::lua::UiEvent;

    use super::Blight;
    use crate::lua::constants::{
        BLIGHT_ON_DIMENSIONS_CHANGE_LISTENER_TABLE, BLIGHT_ON_QUIT_LISTENER_TABLE,
        COMMAND_BINDING_TABLE, COMPLETION_CALLBACK_TABLE, STATUS_AREA_HEIGHT,
    };
    use crate::{PROJECT_NAME, VERSION};

    fn get_lua_state() -> (Lua, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let regex = crate::lua::regex::RegexLib {};
        let blight = Blight::new(writer);
        let lua = Lua::new();
        lua.globals().set("regex", regex).unwrap();
        lua.globals().set("blight", blight).unwrap();
        lua.set_named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(
            BLIGHT_ON_DIMENSIONS_CHANGE_LISTENER_TABLE,
            lua.create_table().unwrap(),
        )
        .unwrap();
        lua.set_named_registry_value(COMPLETION_CALLBACK_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(COMMAND_BINDING_TABLE, lua.create_table().unwrap())
            .unwrap();
        lua.set_named_registry_value(STATUS_AREA_HEIGHT, 1u16)
            .unwrap();
        (lua, reader)
    }

    #[test]
    fn test_config_dir() {
        let (lua, _reader) = get_lua_state();
        assert!(lua
            .load("return blight.config_dir()")
            .call::<String>(())
            .unwrap()
            .ends_with(".run/test/config"));
    }

    #[test]
    fn test_data_dir() {
        let (lua, _reader) = get_lua_state();
        assert!(lua
            .load("return blight.data_dir()")
            .call::<String>(())
            .unwrap()
            .ends_with(".run/test/data"));
    }

    #[test]
    fn test_version() {
        let (lua, _reader) = get_lua_state();
        assert_eq!(
            lua.load("return blight.version()")
                .call::<(String, String)>(())
                .unwrap(),
            (PROJECT_NAME.to_string(), VERSION.to_string())
        );
    }

    #[test]
    fn confirm_on_quite_register() {
        let (lua, _reader) = get_lua_state();
        let table: mlua::Table = lua
            .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
            .unwrap();
        assert_eq!(table.raw_len(), 0);
        lua.load("blight.on_quit(function () end)").exec().unwrap();
        let table: mlua::Table = lua
            .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
            .unwrap();
        assert_eq!(table.raw_len(), 1);
    }

    #[test]
    fn on_complete() {
        let (lua, _reader) = get_lua_state();
        let table: mlua::Table = lua.named_registry_value(COMPLETION_CALLBACK_TABLE).unwrap();
        assert_eq!(table.raw_len(), 0);
        lua.load("blight.on_complete(function () end)")
            .exec()
            .unwrap();
        let table: mlua::Table = lua.named_registry_value(COMPLETION_CALLBACK_TABLE).unwrap();
        assert_eq!(table.raw_len(), 1);
    }

    #[test]
    fn on_quit_function() {
        let (lua, _reader) = get_lua_state();
        lua.load("blight.on_quit(function () blight.output(\"on_quit\") end)")
            .exec()
            .unwrap();
        let table: mlua::Table = lua
            .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)
            .unwrap();
        for pair in table.pairs::<mlua::Value, mlua::Function>() {
            let (_, cb) = pair.unwrap();
            cb.call::<()>(()).unwrap();
        }
        let blight_aux = lua.globals().get::<AnyUserData>("blight").unwrap();
        let mut blight = blight_aux.borrow_mut::<Blight>().unwrap();
        let lines = blight.get_output_lines();
        let mut it = lines.iter();
        assert_eq!(it.next().unwrap(), &crate::model::Line::from("on_quit"));
    }

    #[test]
    fn quit() {
        let (lua, reader) = get_lua_state();
        lua.load("blight.quit()").exec().unwrap();
        assert_eq!(reader.recv(), Ok(Event::Quit(QuitMethod::Script)));
    }

    #[test]
    fn find() {
        let (lua, reader) = get_lua_state();
        let re = crate::model::Regex::new("test", None).unwrap();
        lua.load(r#"blight.find_forward(regex.new("test"))"#)
            .exec()
            .unwrap();
        assert_eq!(reader.recv(), Ok(Event::FindForward(re.clone())));
        lua.load(r#"blight.find_backward(regex.new("test"))"#)
            .exec()
            .unwrap();
        assert_eq!(reader.recv(), Ok(Event::FindBackward(re)));
    }

    #[test]
    fn show_help() {
        let (lua, reader) = get_lua_state();
        lua.load("blight.show_help(\"test1\", false)")
            .exec()
            .unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::ShowHelp("test1".to_string(), false))
        );
        lua.load("blight.show_help(\"test2\", true)")
            .exec()
            .unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::ShowHelp("test2".to_string(), true))
        );
    }

    #[test]
    fn confirm_ui_events() {
        let (lua, _) = get_lua_state();
        lua.load("blight.ui(\"step_left\")").exec().unwrap();
        lua.load("blight.ui(\"step_right\")").exec().unwrap();
        lua.load("blight.ui(\"scroll_up\")").exec().unwrap();
        lua.load("blight.ui(\"scroll_down\")").exec().unwrap();

        let mut blight: Blight = lua.globals().get("blight").unwrap();
        assert_eq!(
            blight.get_ui_events(),
            vec![
                UiEvent::StepLeft,
                UiEvent::StepRight,
                UiEvent::ScrollUp,
                UiEvent::ScrollDown
            ]
        );
    }

    #[test]
    fn test_bad_ui_event() {
        let (lua, reader) = get_lua_state();
        lua.load("blight.ui(\"schplort\")").exec().unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::Error("Invalid ui command: schplort".to_string()))
        );
    }

    #[test]
    fn test_command_bindings() {
        let (lua, _) = get_lua_state();
        lua.load("blight.bind(\"f1\", function () end)")
            .exec()
            .unwrap();
        let bindings: mlua::Table = lua.named_registry_value(COMMAND_BINDING_TABLE).unwrap();
        assert!(bindings.get::<mlua::Function>("f1").is_ok());
        lua.load("blight.unbind(\"f1\")").exec().unwrap();
        assert!(bindings.get::<mlua::Function>("f1").is_err());
    }

    #[test]
    fn test_command_bindings_alt_with_capitalized_letter() {
        let (lua, _) = get_lua_state();
        lua.load("blight.bind(\"Alt-H\", function () end)")
            .exec()
            .unwrap();
        let bindings: mlua::Table = lua.named_registry_value(COMMAND_BINDING_TABLE).unwrap();
        assert!(bindings.get::<mlua::Function>("alt-H").is_ok());
        assert!(bindings.get::<mlua::Function>("alt-h").is_err());
    }

    #[test]
    fn test_status_height() {
        let (lua, _reader) = get_lua_state();
        let height = lua
            .load("return blight.status_height()")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 1);
        let height = lua
            .load("return blight.status_height(3)")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 3);
        let height = lua
            .load("return blight.status_height()")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 3);
        let height = lua
            .load("return blight.status_height(1)")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 1);
        let height = lua
            .load("return blight.status_height()")
            .call::<u16>(())
            .unwrap();
        assert_eq!(height, 1);
    }
}
