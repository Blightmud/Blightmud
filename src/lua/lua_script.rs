use super::{
    audio::Audio, backend::Backend, blight::*, line::Line as LuaLine, plugin, script::Script,
    socket::SocketLib, tts::Tts,
};
use super::{constants::*, core::Core, ui_event::UiEvent};
use super::{
    log::Log, mud::Mud, regex::RegexLib, settings::Settings, store::Store, timer::Timer, util::*,
};
use crate::{event::Event, lua::servers::Servers, model::Line};
use anyhow::Result;
use log::info;
use mlua::{AnyUserData, Lua, Result as LuaResult};
use std::io::prelude::*;
use std::{fs::File, sync::mpsc::Sender};

pub struct LuaScript {
    state: Lua,
    writer: Sender<Event>,
}

fn create_default_lua_state(
    writer: Sender<Event>,
    dimensions: (u16, u16),
    store: Option<Store>,
) -> Lua {
    let state = unsafe { Lua::unsafe_new() };

    let backend = Backend::new(writer.clone());
    let mut blight = Blight::new(writer.clone());
    let store = match store {
        Some(store) => store,
        None => Store::new(),
    };
    let tts = Tts::new();

    blight.screen_dimensions = dimensions;
    blight.core_mode(true);
    let result: LuaResult<()> = (|| {
        let globals = state.globals();

        state.set_named_registry_value(BACKEND, backend)?;
        state.set_named_registry_value(MUD_OUTPUT_LISTENER_TABLE, state.create_table()?)?;
        state.set_named_registry_value(MUD_INPUT_LISTENER_TABLE, state.create_table()?)?;
        state.set_named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE, state.create_table()?)?;
        state.set_named_registry_value(TIMED_CALLBACK_TABLE, state.create_table()?)?;
        state.set_named_registry_value(TIMED_CALLBACK_TABLE_CORE, state.create_table()?)?;
        state.set_named_registry_value(TIMED_NEXT_ID, 1)?;
        state.set_named_registry_value(TIMER_TICK_CALLBACK_TABLE, state.create_table()?)?;
        state.set_named_registry_value(TIMER_TICK_CALLBACK_TABLE_CORE, state.create_table()?)?;
        state.set_named_registry_value(COMMAND_BINDING_TABLE, state.create_table()?)?;
        state.set_named_registry_value(PROTO_ENABLED_LISTENERS_TABLE, state.create_table()?)?;
        state.set_named_registry_value(PROTO_SUBNEG_LISTENERS_TABLE, state.create_table()?)?;
        state.set_named_registry_value(ON_CONNECTION_CALLBACK_TABLE, state.create_table()?)?;
        state.set_named_registry_value(ON_DISCONNECT_CALLBACK_TABLE, state.create_table()?)?;
        state.set_named_registry_value(COMPLETION_CALLBACK_TABLE, state.create_table()?)?;

        globals.set("blight", blight)?;
        globals.set("core", Core::new(writer.clone()))?;
        globals.set("tts", tts)?;
        globals.set("regex", RegexLib {})?;
        globals.set("mud", Mud::new())?;
        globals.set("log", Log::new())?;
        globals.set("timer", Timer::new())?;
        globals.set("script", Script {})?;
        globals.set(Settings::LUA_GLOBAL_NAME, Settings::new())?;
        globals.set(Store::LUA_GLOBAL_NAME, store)?;
        globals.set("plugin", plugin::Handler::new())?;
        globals.set("audio", Audio {})?;
        globals.set("socket", SocketLib {})?;
        globals.set("servers", Servers {})?;

        let lua_json = state
            .load(include_str!("../../resources/lua/json.lua"))
            .call::<_, mlua::Value>(())?;
        globals.set("json", lua_json)?;

        let lua_triggers = state
            .load(include_str!("../../resources/lua/trigger.lua"))
            .call::<_, mlua::Value>(())?;
        globals.set("trigger", lua_triggers)?;

        let lua_aliases = state
            .load(include_str!("../../resources/lua/alias.lua"))
            .call::<_, mlua::Value>(())?;
        globals.set("alias", lua_aliases)?;

        let lua_search = state
            .load(include_str!("../../resources/lua/search.lua"))
            .call::<_, mlua::Value>(())?;
        globals.set("search", lua_search)?;

        state
            .load(include_str!("../../resources/lua/defaults.lua"))
            .exec()?;
        state
            .load(include_str!("../../resources/lua/functions.lua"))
            .exec()?;
        state
            .load(include_str!("../../resources/lua/bindings.lua"))
            .exec()?;
        state
            .load(include_str!("../../resources/lua/lua_command.lua"))
            .exec()?;
        state
            .load(include_str!("../../resources/lua/macros.lua"))
            .exec()?;
        state
            .load(include_str!("../../resources/lua/plugins.lua"))
            .exec()?;

        let lua_gmcp = state
            .load(include_str!("../../resources/lua/gmcp.lua"))
            .call::<_, mlua::Value>(())?;
        globals.set("gmcp", lua_gmcp)?;
        let lua_msdp = state
            .load(include_str!("../../resources/lua/msdp.lua"))
            .call::<_, mlua::Value>(())?;
        globals.set("msdp", lua_msdp)?;
        let lua_tasks = state
            .load(include_str!("../../resources/lua/tasks.lua"))
            .call::<_, mlua::Value>(())?;
        globals.set("tasks", lua_tasks)?;

        {
            let blight_aud: AnyUserData = globals.get("blight")?;
            let mut blight = blight_aud.borrow_mut::<Blight>()?;
            blight.core_mode(false);
        }

        state
            .load(include_str!("../../resources/lua/on_state_created.lua"))
            .exec()?;

        Ok(())
    })();

    if let Err(err) = result {
        output_stack_trace(&writer, &err.to_string());
    }

    state
}

impl LuaScript {
    pub fn new(main_writer: Sender<Event>, dimensions: (u16, u16)) -> Self {
        Self {
            state: create_default_lua_state(main_writer.clone(), dimensions, None),
            writer: main_writer,
        }
    }

    pub fn reset(&mut self, dimensions: (u16, u16)) {
        let store = self.state.globals().get(Store::LUA_GLOBAL_NAME).ok();
        self.state = create_default_lua_state(self.writer.clone(), dimensions, store);
    }

    pub fn get_output_lines(&self) -> Vec<Line> {
        let blight_aud: AnyUserData = self
            .state
            .globals()
            .get("blight")
            .expect("blight global not found");
        let mut blight = blight_aud
            .borrow_mut::<Blight>()
            .expect("Could not borrow blight global as mut");
        blight.get_output_lines()
    }

    fn exec_lua<T>(&self, func: &mut dyn FnMut() -> LuaResult<T>) -> Option<T> {
        let result = func();
        if let Err(msg) = &result {
            output_stack_trace(&self.writer, &msg.to_string());
        }
        result.ok()
    }

    pub fn on_mud_output(&self, line: &mut Line) {
        if !line.flags.bypass_script {
            let mut lline = LuaLine::from(line.clone());
            self.exec_lua(&mut || -> LuaResult<()> {
                let table: mlua::Table =
                    self.state.named_registry_value(MUD_OUTPUT_LISTENER_TABLE)?;
                for pair in table.pairs::<mlua::Value, mlua::Function>() {
                    let (_, cb) = pair?;
                    lline = cb.call::<_, LuaLine>(lline.clone())?;
                }
                line.replace_with(&lline.inner);
                if let Some(replacement) = &lline.replacement {
                    line.set_content(replacement);
                }
                Ok(())
            });
        }
    }

    pub fn on_mud_input(&self, line: &mut Line) {
        if !line.flags.bypass_script {
            let mut lline = LuaLine::from(line.clone());
            self.exec_lua(&mut || -> LuaResult<()> {
                let table: mlua::Table =
                    self.state.named_registry_value(MUD_INPUT_LISTENER_TABLE)?;
                for pair in table.pairs::<mlua::Value, mlua::Function>() {
                    let (_, cb) = pair?;
                    lline = cb.call::<_, LuaLine>(lline.clone())?;
                }
                line.replace_with(&lline.inner);
                if let Some(replacement) = &lline.replacement {
                    line.set_content(replacement);
                }
                Ok(())
            });
        }
    }

    pub fn on_quit(&self) {
        self.exec_lua(&mut || -> LuaResult<()> {
            let table: mlua::Table = self
                .state
                .named_registry_value(BLIGHT_ON_QUIT_LISTENER_TABLE)?;
            for pair in table.pairs::<mlua::Value, mlua::Function>() {
                let (_, cb) = pair?;
                cb.call::<_, ()>(())?;
            }
            Ok(())
        });
    }

    pub fn run_timed_function(&mut self, id: u32) {
        self.exec_lua(&mut || -> LuaResult<()> {
            let core_table: mlua::Table =
                self.state.named_registry_value(TIMED_CALLBACK_TABLE_CORE)?;
            match core_table.get(id)? {
                mlua::Value::Function(func) => func.call::<_, ()>(()),
                _ => {
                    let table: mlua::Table =
                        self.state.named_registry_value(TIMED_CALLBACK_TABLE)?;
                    match table.get(id)? {
                        mlua::Value::Function(func) => func.call::<_, ()>(()),
                        _ => Ok(()),
                    }
                }
            }
        });
    }

    pub fn tick(&mut self, millis: u128) {
        self.exec_lua(&mut || -> LuaResult<()> {
            let core_tick_table: mlua::Table = self
                .state
                .named_registry_value(TIMER_TICK_CALLBACK_TABLE_CORE)?;
            let tick_table: mlua::Table =
                self.state.named_registry_value(TIMER_TICK_CALLBACK_TABLE)?;
            let pairs = core_tick_table
                .pairs::<mlua::Integer, mlua::Function>()
                .chain(tick_table.pairs::<mlua::Integer, mlua::Function>());
            for pair in pairs.flatten() {
                pair.1.call::<_, ()>(millis)?;
            }

            Ok(())
        });
    }

    pub fn remove_timed_function(&mut self, id: u32) {
        self.exec_lua(&mut || -> LuaResult<()> {
            let core_table: mlua::Table =
                self.state.named_registry_value(TIMED_CALLBACK_TABLE_CORE)?;
            let table: mlua::Table = self.state.named_registry_value(TIMED_CALLBACK_TABLE)?;
            if let Err(core_err) = core_table.set(id, mlua::Nil) {
                return Err(core_err);
            }
            table.set(id, mlua::Nil)
        });
    }

    pub fn load_script(&mut self, path: &str) -> Result<()> {
        info!("Loading: {}", path);
        let file_path = expand_tilde(path);
        let mut file = File::open(file_path.as_ref())?;
        let dir = file_path.rsplitn(2, '/').nth(1).unwrap_or("");
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        self.exec_lua(&mut || -> LuaResult<()> {
            let package: mlua::Table = self.state.globals().get("package")?;
            let ppath = package.get::<&str, String>("path")?;
            package.set("path", format!("{0}/?.lua;{1}", dir, ppath))?;
            let result = self.state.load(&content).set_name(dir)?.exec();
            package.set("path", ppath)?;
            result
        });
        Ok(())
    }

    pub fn on_connect(&mut self, host: &str, port: u16, id: u16) {
        self.exec_lua(&mut || -> LuaResult<()> {
            self.state.set_named_registry_value(CONNECTION_ID, id)?;
            let table: mlua::Table = self
                .state
                .named_registry_value(ON_CONNECTION_CALLBACK_TABLE)?;
            for pair in table.pairs::<mlua::Value, mlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>((host, port))?;
            }
            Ok(())
        });
    }

    pub fn on_disconnect(&mut self) {
        self.exec_lua(&mut || -> LuaResult<()> {
            let table: mlua::Table = self
                .state
                .named_registry_value(ON_DISCONNECT_CALLBACK_TABLE)?;
            for pair in table.pairs::<mlua::Value, mlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>(())?;
            }
            Ok(())
        });
    }

    pub fn set_dimensions(&mut self, dim: (u16, u16)) {
        self.exec_lua(&mut || -> LuaResult<()> {
            let blight_aud: AnyUserData = self.state.globals().get("blight")?;
            let mut blight = blight_aud.borrow_mut::<Blight>()?;
            blight.screen_dimensions = dim;
            Ok(())
        });
    }

    pub fn proto_enabled(&mut self, proto: u8) {
        self.exec_lua(&mut || -> LuaResult<()> {
            let table: mlua::Table = self
                .state
                .named_registry_value(PROTO_ENABLED_LISTENERS_TABLE)?;
            for pair in table.pairs::<mlua::Value, mlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>(proto)?;
            }
            Ok(())
        });
    }

    pub fn proto_subneg(&mut self, proto: u8, bytes: &[u8]) {
        self.exec_lua(&mut || -> LuaResult<()> {
            let table: mlua::Table = self
                .state
                .named_registry_value(PROTO_SUBNEG_LISTENERS_TABLE)?;
            for pair in table.pairs::<mlua::Value, mlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>((proto, bytes.to_vec()))?;
            }
            Ok(())
        });
    }

    pub fn tab_complete(&mut self, input: &str) -> Option<Vec<String>> {
        self.exec_lua(&mut || -> LuaResult<Option<Vec<String>>> {
            let cb_table: mlua::Table =
                self.state.named_registry_value(COMPLETION_CALLBACK_TABLE)?;
            let mut completions = vec![];
            for cb in cb_table.sequence_values::<mlua::Function>() {
                let cb = cb?;
                let result = cb.call::<_, Option<Vec<String>>>(input.to_string())?;
                if let Some(mut options) = result {
                    options.sort();
                    completions.append(&mut options);
                }
            }
            Ok(if completions.is_empty() {
                None
            } else {
                Some(completions)
            })
        })
        .unwrap_or(None)
    }

    pub fn check_bindings(&mut self, cmd: &str) -> bool {
        let mut response = false;
        self.exec_lua(&mut || -> LuaResult<()> {
            let bind_table: mlua::Table = self.state.named_registry_value(COMMAND_BINDING_TABLE)?;
            if let Ok(callback) = bind_table.get::<_, mlua::Function>(cmd) {
                response = true;
                callback.call::<_, ()>(())
            } else {
                Ok(())
            }
        });
        response
    }

    pub fn get_ui_events(&mut self) -> Vec<UiEvent> {
        match (|| -> LuaResult<Vec<UiEvent>> {
            let blight_aud: AnyUserData = self.state.globals().get("blight")?;
            let mut blight = blight_aud.borrow_mut::<Blight>()?;
            let events = blight.get_ui_events();
            Ok(events)
        })() {
            Ok(data) => data,
            Err(msg) => {
                output_stack_trace(&self.writer, &msg.to_string());
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod lua_script_tests {
    use super::LuaScript;
    use super::CONNECTION_ID;
    use crate::event::QuitMethod;
    use crate::lua::constants::TIMED_CALLBACK_TABLE;
    use crate::model::{Connection, Regex};
    use crate::{event::Event, lua::regex::Regex as LReg, model::Line, PROJECT_NAME, VERSION};
    use std::{
        collections::BTreeMap,
        sync::mpsc::{channel, Receiver, Sender},
    };

    fn test_trigger(line: &str, lua: &LuaScript) -> bool {
        let mut line = Line::from(line);
        lua.on_mud_output(&mut line);
        line.flags.matched
    }

    fn test_prompt_trigger(line: &str, lua: &LuaScript) -> bool {
        let mut line = Line::from(line);
        line.flags.prompt = true;
        lua.on_mud_output(&mut line);
        line.flags.matched
    }

    fn get_lua() -> (LuaScript, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let lua = LuaScript::new(writer, (80, 80));
        loop {
            if reader.try_recv().is_err() {
                break;
            }
        }
        (lua, reader)
    }

    #[test]
    fn test_lua_trigger() {
        let create_trigger_lua = r#"
        trigger.add("^test$", {gag=true}, function () end)
        "#;

        let lua = get_lua().0;
        lua.state.load(create_trigger_lua).exec().unwrap();

        assert!(test_trigger("test", &lua));
        assert!(!test_trigger("test test", &lua));
    }

    #[test]
    fn test_lua_counted_trigger() {
        let create_trigger_lua = r#"
        trigger.add("^test$", {count=3}, function () end)
        "#;

        let lua = get_lua().0;
        lua.state.load(create_trigger_lua).exec().unwrap();

        assert!(test_trigger("test", &lua));
        assert!(test_trigger("test", &lua));
        assert!(test_trigger("test", &lua));
        assert!(!test_trigger("test", &lua));
    }

    #[test]
    fn test_lua_prompt_trigger() {
        let create_prompt_trigger_lua = r#"
        trigger.add("^test$", {prompt=true, gag=true}, function () end)
        "#;

        let lua = get_lua().0;
        lua.state.load(create_prompt_trigger_lua).exec().unwrap();

        assert!(test_prompt_trigger("test", &lua));
        assert!(!test_prompt_trigger("test test", &lua));
    }

    #[test]
    fn test_lua_trigger_id_increment() {
        let lua = get_lua().0;
        lua.state
            .load(r#"trigger.add("^test regular$", {}, function () end)"#)
            .exec()
            .unwrap();
        lua.state
            .load(r#"trigger.add("^test regular$", {}, function () end)"#)
            .exec()
            .unwrap();
        let ttrig: u32 = lua
            .state
            .load(r#"return trigger.add("^test$", {}, function () end).id"#)
            .call(())
            .unwrap();
        let ptrig: u32 = lua
            .state
            .load(r#"return trigger.add("^test$", {prompt=true}, function () end).id"#)
            .call(())
            .unwrap();

        assert_ne!(ttrig, ptrig);
    }

    #[test]
    fn test_lua_raw_trigger() {
        let create_trigger_lua = r#"
        trigger.add("^\\x1b\\[31mtest\\x1b\\[0m$", {raw=true}, function () end)
        "#;

        let (lua, _reader) = get_lua();
        lua.state.load(create_trigger_lua).exec().unwrap();

        assert!(test_trigger("\x1b[31mtest\x1b[0m", &lua));
        assert!(!test_trigger("test", &lua));
    }

    #[test]
    fn test_remove_trigger() {
        let lua = get_lua().0;
        let ttrig: u32 = lua
            .state
            .load(r#"return trigger.add("^test$", {}, function () end).id"#)
            .call(())
            .unwrap();
        let ptrig: u32 = lua
            .state
            .load(r#"return trigger.add("^test$", {prompt=true}, function () end).id"#)
            .call(())
            .unwrap();

        assert!(test_trigger("test", &lua));
        assert!(test_prompt_trigger("test", &lua));

        lua.state
            .load(&format!("trigger.remove({})", ttrig))
            .exec()
            .unwrap();

        assert!(test_prompt_trigger("test", &lua));
        assert!(!test_trigger("test", &lua));

        lua.state
            .load(&format!("trigger.remove({})", ptrig))
            .exec()
            .unwrap();

        assert!(!test_trigger("test", &lua));
        assert!(!test_prompt_trigger("test", &lua));
    }

    fn check_alias_match(lua: &LuaScript, mut line: Line) -> bool {
        lua.on_mud_input(&mut line);
        line.flags.matched
    }

    #[test]
    fn test_lua_alias() {
        let create_alias_lua = r#"
        alias.add("^test$", function () end)
        "#;

        let lua = get_lua().0;
        lua.state.load(create_alias_lua).exec().unwrap();

        assert!(check_alias_match(&lua, Line::from("test")));
        assert!(!check_alias_match(&lua, Line::from(" test")));
    }

    #[test]
    fn test_lua_remove_alias() {
        let create_alias_lua = r#"
        return alias.add("^test$", function () end).id
        "#;

        let lua = get_lua().0;
        let index: i32 = lua.state.load(create_alias_lua).call(()).unwrap();

        assert!(check_alias_match(&lua, Line::from("test")));

        let delete_alias = format!("alias.remove({})", index);
        lua.state.load(&delete_alias).exec().unwrap();
        assert!(!check_alias_match(&lua, Line::from("test")));
    }

    #[test]
    fn test_dimensions() {
        let mut lua = get_lua().0;
        let dim: (u16, u16) = lua
            .state
            .load("return blight.terminal_dimensions()")
            .call(())
            .unwrap();
        assert_eq!(dim, (80, 80));
        lua.set_dimensions((70, 70));
        let dim: (u16, u16) = lua
            .state
            .load("return blight.terminal_dimensions()")
            .call(())
            .unwrap();
        assert_eq!(dim, (70, 70));
    }

    #[test]
    fn test_enable_proto() {
        let send_gmcp_lua = r#"
        core.enable_protocol(200)
        "#;

        let (lua, reader) = get_lua();
        lua.state.load(send_gmcp_lua).exec().unwrap();

        assert_eq!(reader.recv(), Ok(Event::EnableProto(200)));
    }

    #[test]
    fn test_proto_send() {
        let send_gmcp_lua = r#"
        core.subneg_send(201, { 255, 250, 86, 255, 240 })
        "#;

        let (lua, reader) = get_lua();
        lua.state.load(send_gmcp_lua).exec().unwrap();

        assert_eq!(
            reader.recv(),
            Ok(Event::ProtoSubnegSend(201, vec![255, 250, 86, 255, 240]))
        );
    }

    #[test]
    fn test_version() {
        let lua = get_lua().0;
        let (name, version): (String, String) = lua
            .state
            .load("return blight.version()")
            .call::<(), (String, String)>(())
            .unwrap();
        assert_eq!(version, VERSION);
        assert_eq!(name, PROJECT_NAME);
    }

    fn assert_event(lua_code: &str, event: Event) {
        let (lua, reader) = get_lua();
        lua.state.load(lua_code).exec().unwrap();

        assert_eq!(reader.recv(), Ok(event));
    }

    fn assert_events(lua_code: &str, events: Vec<Event>) {
        let (lua, reader) = get_lua();
        lua.state.load(lua_code).exec().unwrap();

        for event in events.iter() {
            assert_eq!(reader.recv(), Ok(event.clone()));
        }
    }

    #[test]
    fn test_output() {
        let (lua, _) = get_lua();
        lua.state
            .load("blight.output(\"test\", \"test\")")
            .exec()
            .unwrap();
        assert_eq!(lua.get_output_lines(), vec![Line::from("test test")]);
    }

    #[test]
    fn test_load() {
        assert_event(
            "script.load(\"/some/fancy/path\")",
            Event::LoadScript("/some/fancy/path".to_string()),
        );
    }

    #[test]
    fn test_reset() {
        assert_event("script.reset()", Event::ResetScript);
    }

    #[test]
    fn test_sending() {
        assert_events(
            "mud.send(\"message\")",
            vec![Event::ServerInput(Line::from("message"))],
        );
    }

    #[test]
    fn test_conditional_gag() {
        let trigger = r#"
        trigger.add("^Health (\\d+)$", {}, function (matches, line)
            if matches[2] == "100" then
                line:gag(true)
            end
        end)
        "#;

        let (lua, _reader) = get_lua();
        lua.state.load(trigger).exec().unwrap();

        let mut line = Line::from("Health 100");
        lua.on_mud_output(&mut line);
        assert!(line.flags.gag);

        let mut line = Line::from("Health 10");
        lua.on_mud_output(&mut line);
        assert!(!line.flags.gag);
    }

    fn check_color(lua: &LuaScript, output: &str, result: &str) {
        lua.state
            .load(&format!("blight.output({})", output))
            .exec()
            .unwrap();
        assert_eq!(lua.get_output_lines()[0], Line::from(result));
    }

    #[test]
    fn test_color_output() {
        let (lua, _reader) = get_lua();
        check_color(
            &lua,
            "C_RED .. \"COLOR\" .. C_RESET",
            "\x1b[31mCOLOR\x1b[0m",
        );
        check_color(
            &lua,
            "C_GREEN .. \"COLOR\" .. C_RESET",
            "\x1b[32mCOLOR\x1b[0m",
        );
        check_color(
            &lua,
            "C_YELLOW .. \"COLOR\" .. C_RESET",
            "\x1b[33mCOLOR\x1b[0m",
        );
        check_color(
            &lua,
            "C_BLUE .. \"COLOR\" .. C_RESET",
            "\x1b[34mCOLOR\x1b[0m",
        );
        check_color(
            &lua,
            "C_MAGENTA .. \"COLOR\" .. C_RESET",
            "\x1b[35mCOLOR\x1b[0m",
        );
        check_color(
            &lua,
            "C_CYAN .. \"COLOR\" .. C_RESET",
            "\x1b[36mCOLOR\x1b[0m",
        );
        check_color(
            &lua,
            "C_WHITE .. \"COLOR\" .. C_RESET",
            "\x1b[37mCOLOR\x1b[0m",
        );
    }

    #[test]
    fn test_bindings() {
        let lua_code = r#"
        blight.bind("ctrl-a", function ()
            blight.output("ctrl-a")
        end)
        blight.bind("f1", function ()
            blight.output("f1")
        end)
        blight.bind("alt-1", function ()
            blight.output("alt-1")
        end)
        blight.bind("\x1b[1;5A", function ()
            blight.output("ctrl-up")
        end)
        "#;

        let (mut lua, _reader) = get_lua();
        lua.state.load(lua_code).exec().unwrap();

        lua.check_bindings("ctrl-a");
        assert_eq!(lua.get_output_lines(), [Line::from("ctrl-a")]);
        lua.check_bindings("alt-1");
        assert_eq!(lua.get_output_lines(), [Line::from("alt-1")]);
        lua.check_bindings("f1");
        assert_eq!(lua.get_output_lines(), [Line::from("f1")]);
        lua.check_bindings("ctrl-0");
        assert_eq!(lua.get_output_lines(), []);
        lua.check_bindings("\x1b[1;5a");
        assert_eq!(lua.get_output_lines(), [Line::from("ctrl-up")]);
    }

    #[test]
    fn test_on_connect_test() {
        let lua_code = r#"
        mud.on_connect(function (host, port)
            blight.output(host .. ":" .. port .. "-1")
        end)
        mud.on_connect(function (host, port)
            blight.output(host .. ":" .. port .. "-2")
        end)
        mud.on_connect(function (host, port)
            blight.output(host .. ":" .. port .. "-3")
        end)
        "#;

        let (mut lua, _reader) = get_lua();
        lua.state.load(lua_code).exec().unwrap();

        lua.on_connect("test", 21, 12);
        assert_eq!(
            lua.get_output_lines(),
            [
                Line::from("test:21-1"),
                Line::from("test:21-2"),
                Line::from("test:21-3"),
            ]
        );
        assert_eq!(
            lua.state
                .named_registry_value::<_, u32>(CONNECTION_ID)
                .unwrap(),
            12
        );
        lua.reset((100, 100));
        lua.state.load(lua_code).exec().unwrap();
        lua.on_connect("server", 1000, 13);
        assert_eq!(
            lua.get_output_lines(),
            [
                Line::from("server:1000-1"),
                Line::from("server:1000-2"),
                Line::from("server:1000-3"),
            ]
        );
        assert_eq!(
            lua.state
                .named_registry_value::<_, u32>(CONNECTION_ID)
                .unwrap(),
            13
        );
    }

    #[test]
    fn test_on_disconnect_test() {
        let lua_code = r#"
        mud.on_disconnect(function ()
            blight.output("disconnected1")
        end)
        mud.on_disconnect(function ()
            blight.output("disconnected2")
        end)
        mud.on_disconnect(function ()
            blight.output("disconnected3")
        end)
        "#;

        let (mut lua, _reader) = get_lua();
        lua.state.load(lua_code).exec().unwrap();

        lua.on_disconnect();
        assert_eq!(
            lua.get_output_lines(),
            [
                Line::from("disconnected1"),
                Line::from("disconnected2"),
                Line::from("disconnected3"),
            ]
        );
        lua.reset((100, 100));
        lua.state.load(lua_code).exec().unwrap();
        lua.on_disconnect();
        assert_eq!(
            lua.get_output_lines(),
            [
                Line::from("disconnected1"),
                Line::from("disconnected2"),
                Line::from("disconnected3"),
            ]
        );
    }

    #[test]
    fn test_alias_ids() {
        let (lua, _reader) = get_lua();
        let id = lua
            .state
            .load(r#"return alias.add("test", function () end).id"#)
            .call(())
            .unwrap();

        let aliases: BTreeMap<u32, mlua::Table> = lua
            .state
            .load(r#"return alias.get_group():get_aliases()"#)
            .call(())
            .unwrap();

        assert!(aliases.contains_key(&id));

        let alias: &mlua::Table = aliases.get(&id).unwrap();
        assert_eq!(alias.get::<_, bool>("enabled").unwrap(), true);
        assert_eq!(alias.get::<_, LReg>("regex").unwrap().to_string(), "test");

        lua.state.load(r#"alias.clear()"#).exec().unwrap();
        let ids: BTreeMap<u32, mlua::Table> = lua
            .state
            .load(r#"return alias.get_group():get_aliases()"#)
            .call(())
            .unwrap();

        assert!(ids.is_empty());
    }

    #[test]
    fn test_trigger_ids() {
        let (lua, _reader) = get_lua();
        let id = lua
            .state
            .load(r#"return trigger.add("test", {}, function () end).id"#)
            .call(())
            .unwrap();

        let triggers: BTreeMap<u32, mlua::Table> = lua
            .state
            .load(r#"return trigger.get_group():get_triggers()"#)
            .call(())
            .unwrap();

        assert!(triggers.contains_key(&id));

        let trigger: &mlua::Table = triggers.get(&id).unwrap();
        assert_eq!(trigger.get::<_, LReg>("regex").unwrap().to_string(), "test");
        assert_eq!(trigger.get::<_, bool>("enabled").unwrap(), true);
        assert_eq!(trigger.get::<_, bool>("gag").unwrap(), false);
        assert_eq!(trigger.get::<_, bool>("raw").unwrap(), false);
        assert_eq!(trigger.get::<_, bool>("prompt").unwrap(), false);

        lua.state.load(r#"trigger.clear()"#).exec().unwrap();
        let ids: BTreeMap<u32, mlua::Table> = lua
            .state
            .load(r#"return trigger.get_group():get_triggers()"#)
            .call(())
            .unwrap();

        assert!(ids.is_empty());
    }

    #[test]
    fn confirm_connection_macros() {
        let (lua, reader) = get_lua();
        lua.on_mud_input(&mut Line::from("/connect example.com 4000"));
        assert_eq!(
            reader.recv().unwrap(),
            Event::Connect(Connection::new("example.com", 4000, false, false))
        );
        lua.on_mud_input(&mut Line::from("/connect example.com 4000 true"));
        assert_eq!(
            reader.recv().unwrap(),
            Event::Connect(Connection::new("example.com", 4000, true, true))
        );
        lua.on_mud_input(&mut Line::from("/connect example.com 4000 true true"));
        assert_eq!(
            reader.recv().unwrap(),
            Event::Connect(Connection::new("example.com", 4000, true, true))
        );
        lua.on_mud_input(&mut Line::from("/connect example.com 4000 true false"));
        assert_eq!(
            reader.recv().unwrap(),
            Event::Connect(Connection::new("example.com", 4000, true, false))
        );
        lua.state
            .set_named_registry_value(CONNECTION_ID, 4)
            .unwrap();
        lua.on_mud_input(&mut Line::from("/disconnect"));
        assert_eq!(reader.recv().unwrap(), Event::Disconnect(4));

        lua.on_mud_input(&mut Line::from("/reconnect"));
        assert_eq!(reader.recv().unwrap(), Event::Reconnect);
    }

    #[test]
    fn confirm_logging_macros() {
        let (lua, reader) = get_lua();
        lua.on_mud_input(&mut Line::from("/start_log test"));
        assert_eq!(
            reader.recv().unwrap(),
            Event::StartLogging("test".to_string(), true)
        );
        lua.on_mud_input(&mut Line::from("/stop_log"));
        assert_eq!(reader.recv().unwrap(), Event::StopLogging);
    }

    #[test]
    fn confirm_load_macro() {
        let (lua, reader) = get_lua();
        lua.on_mud_input(&mut Line::from("/load test"));
        assert_eq!(
            reader.recv().unwrap(),
            Event::LoadScript("test".to_string())
        );
    }

    #[test]
    fn confirm_quit_macro() {
        let (lua, reader) = get_lua();
        lua.on_mud_input(&mut Line::from("/quit"));
        assert_eq!(reader.recv().unwrap(), Event::Quit(QuitMethod::Script));
        lua.on_mud_input(&mut Line::from("/q"));
        assert_eq!(reader.recv().unwrap(), Event::Quit(QuitMethod::Script));
    }

    #[test]
    fn confirm_help_macro() {
        let (lua, reader) = get_lua();
        lua.on_mud_input(&mut Line::from("/help test1"));
        assert_eq!(
            reader.recv().unwrap(),
            Event::ShowHelp("test1".to_string(), true)
        );
    }

    #[test]
    fn confirm_search_macros() {
        let (lua, reader) = get_lua();
        lua.on_mud_input(&mut Line::from("/search test1"));
        let re = Regex::new("test1").unwrap();
        assert_eq!(reader.recv().unwrap(), Event::FindBackward(re.clone()));
        lua.on_mud_input(&mut Line::from("/s test1"));
        assert_eq!(reader.recv().unwrap(), Event::FindBackward(re));
    }

    #[test]
    fn confirm_tick_callback() {
        let (mut lua, _reader) = get_lua();
        lua.state
            .load(
                r#"
        total_millis = 0
        timer.on_tick(function (millis) total_millis = total_millis + millis end)
        "#,
            )
            .exec()
            .unwrap();
        lua.tick(100);
        assert_eq!(
            lua.state.globals().get::<_, u128>("total_millis").unwrap(),
            100
        );
        lua.tick(100);
        assert_eq!(
            lua.state.globals().get::<_, u128>("total_millis").unwrap(),
            200
        );
        lua.tick(100);
        assert_eq!(
            lua.state.globals().get::<_, u128>("total_millis").unwrap(),
            300
        );
    }

    #[test]
    fn confirm_quit_callback() {
        let (lua, _reader) = get_lua();
        lua.state
            .load(
                r#"
        quit = false
        blight.on_quit(function () quit = true end)
        "#,
            )
            .exec()
            .unwrap();
        assert!(!lua.state.globals().get::<_, bool>("quit").unwrap());
        lua.on_quit();
        assert!(lua.state.globals().get::<_, bool>("quit").unwrap());
    }

    #[test]
    fn confirm_timed_function() {
        let (mut lua, _reader) = get_lua();
        lua.state
            .load(
                r#"
        run = false
        id = timer.add(1, 1, function () run = true end)
        "#,
            )
            .exec()
            .unwrap();
        assert!(!lua.state.globals().get::<_, bool>("run").unwrap());
        let id = lua.state.globals().get::<_, u32>("id").unwrap();
        lua.run_timed_function(id);
        assert!(lua.state.globals().get::<_, bool>("run").unwrap());
    }

    #[test]
    fn confirm_remove_timed_function() {
        let (mut lua, _reader) = get_lua();
        lua.state
            .load(
                r#"
        id = timer.add(1, 1, function () run = true end)
        "#,
            )
            .exec()
            .unwrap();
        let id = lua.state.globals().get::<_, u32>("id").unwrap();
        assert!(lua
            .state
            .named_registry_value::<_, mlua::Table>(TIMED_CALLBACK_TABLE)
            .unwrap()
            .contains_key(id)
            .unwrap());
        lua.remove_timed_function(id);
        assert!(!lua
            .state
            .named_registry_value::<_, mlua::Table>(TIMED_CALLBACK_TABLE)
            .unwrap()
            .contains_key(id)
            .unwrap());
    }

    #[test]
    fn confirm_proto_enabled() {
        let (mut lua, _reader) = get_lua();
        lua.state
            .load(
                r#"
        subneg = 0
        core.on_protocol_enabled(function (proto) subneg = proto end)
        "#,
            )
            .exec()
            .unwrap();
        lua.proto_enabled(201);
        assert_eq!(lua.state.globals().get::<_, u32>("subneg").unwrap(), 201);
    }

    #[test]
    fn confirm_proto_subneg() {
        let (mut lua, _reader) = get_lua();
        lua.state
            .load(
                r#"
        subneg = 0
        core.subneg_recv(function (proto, _) subneg = proto end)
        "#,
            )
            .exec()
            .unwrap();
        lua.proto_subneg(201, &[]);
        assert_eq!(lua.state.globals().get::<_, u32>("subneg").unwrap(), 201);
    }

    #[test]
    fn confirm_completion() {
        let (mut lua, _reader) = get_lua();
        lua.state
            .load(
                r#"
                blight.on_complete(function (input)
                    if input == "bat" then
                        return {"batman"}
                    elseif input == "batm" then
                        return {"batman", "batmobile"}
                    else
                        return nil
                    end
                end)
                "#,
            )
            .exec()
            .unwrap();

        assert_eq!(
            lua.tab_complete(&"bat".to_string()),
            Some(vec!["batman".to_string()])
        );
        assert_eq!(
            lua.tab_complete(&"batm".to_string()),
            Some(vec!["batman".to_string(), "batmobile".to_string()])
        );
        assert_eq!(lua.tab_complete(&"rob".to_string()), None);
    }
}
