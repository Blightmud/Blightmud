use super::{blight::*, line::Line as LuaLine, tts::Tts, util::expand_tilde};
use super::{constants::*, core::Core, ui_event::UiEvent};
use super::{mud::Mud, regex::RegexLib, util::*};
use crate::{event::Event, model::Line};
use anyhow::Result;
use rlua::{Lua, Result as LuaResult};
use std::io::prelude::*;
use std::{fs::File, sync::mpsc::Sender};

pub struct LuaScript {
    state: Lua,
    writer: Sender<Event>,
}

fn create_default_lua_state(
    writer: Sender<Event>,
    dimensions: (u16, u16),
    core: Option<Core>,
) -> Lua {
    let state = unsafe { Lua::new_with_debug() };

    let mut blight = Blight::new(writer.clone());
    let core = match core {
        Some(core) => core,
        None => Core::new(writer.clone()),
    };
    let tts = Tts::new(writer.clone());

    blight.screen_dimensions = dimensions;
    blight.core_mode(true);
    let result = state.context(|ctx| -> LuaResult<()> {
        let globals = ctx.globals();
        globals.set("blight", blight)?;
        globals.set("core", core)?;
        globals.set("tts", tts)?;
        globals.set("regex", RegexLib {})?;
        globals.set("mud", Mud::new())?;

        globals.set(MUD_OUTPUT_LISTENER_TABLE, ctx.create_table()?)?;
        globals.set(MUD_INPUT_LISTENER_TABLE, ctx.create_table()?)?;

        globals.set(TIMED_FUNCTION_TABLE, ctx.create_table()?)?;
        globals.set(TIMED_FUNCTION_TABLE_CORE, ctx.create_table()?)?;
        globals.set(COMMAND_BINDING_TABLE, ctx.create_table()?)?;
        globals.set(PROTO_ENABLED_LISTENERS_TABLE, ctx.create_table()?)?;
        globals.set(PROTO_SUBNEG_LISTENERS_TABLE, ctx.create_table()?)?;
        globals.set(ON_CONNECTION_CALLBACK_TABLE, ctx.create_table()?)?;
        globals.set(ON_DISCONNECT_CALLBACK_TABLE, ctx.create_table()?)?;

        let lua_json = ctx
            .load(include_str!("../../resources/lua/json.lua"))
            .call::<_, rlua::Value>(())?;
        globals.set("json", lua_json)?;

        let lua_triggers = ctx
            .load(include_str!("../../resources/lua/trigger.lua"))
            .call::<_, rlua::Value>(())?;
        globals.set("trigger", lua_triggers)?;

        let lua_aliases = ctx
            .load(include_str!("../../resources/lua/alias.lua"))
            .call::<_, rlua::Value>(())?;
        globals.set("alias", lua_aliases)?;

        ctx.load(include_str!("../../resources/lua/defaults.lua"))
            .exec()?;
        ctx.load(include_str!("../../resources/lua/functions.lua"))
            .exec()?;
        ctx.load(include_str!("../../resources/lua/bindings.lua"))
            .exec()?;
        ctx.load(include_str!("../../resources/lua/lua_command.lua"))
            .exec()?;
        ctx.load(include_str!("../../resources/lua/macros.lua"))
            .exec()?;

        let lua_gmcp = ctx
            .load(include_str!("../../resources/lua/gmcp.lua"))
            .call::<_, rlua::Value>(())?;
        globals.set("gmcp", lua_gmcp)?;
        let lua_msdp = ctx
            .load(include_str!("../../resources/lua/msdp.lua"))
            .call::<_, rlua::Value>(())?;
        globals.set("msdp", lua_msdp)?;
        let lua_tasks = ctx
            .load(include_str!("../../resources/lua/tasks.lua"))
            .call::<_, rlua::Value>(())?;
        globals.set("tasks", lua_tasks)?;

        let mut blight: Blight = globals.get("blight")?;
        blight.core_mode(false);
        globals.set("blight", blight)?;

        Ok(())
    });

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
        let core = self
            .state
            .context(|ctx| -> Result<Core, rlua::Error> { ctx.globals().get("core") })
            .ok();
        self.state = create_default_lua_state(self.writer.clone(), dimensions, core);
    }

    pub fn get_output_lines(&self) -> Vec<Line> {
        self.state
            .context(|ctx| -> LuaResult<Vec<Line>> {
                let mut blight: Blight = ctx.globals().get("blight")?;
                let lines = blight.get_output_lines();
                ctx.globals().set("blight", blight)?;
                Ok(lines)
            })
            .unwrap()
    }

    pub fn on_mud_output(&self, line: &mut Line) {
        let mut lline = LuaLine::from(line.clone());
        if let Err(msg) = self.state.context(|ctx| -> rlua::Result<()> {
            let table: rlua::Table = ctx.globals().get(MUD_OUTPUT_LISTENER_TABLE)?;
            for pair in table.pairs::<rlua::Value, rlua::Function>() {
                let (_, cb) = pair?;
                lline = cb.call::<_, LuaLine>(lline)?;
            }
            line.replace_with(&lline.inner);
            Ok(())
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn on_mud_input(&self, line: &mut Line) {
        let mut lline = LuaLine::from(line.clone());
        if let Err(msg) = self.state.context(|ctx| -> rlua::Result<()> {
            let table: rlua::Table = ctx.globals().get(MUD_INPUT_LISTENER_TABLE)?;
            for pair in table.pairs::<rlua::Value, rlua::Function>() {
                let (_, cb) = pair?;
                lline = cb.call::<_, LuaLine>(lline)?;
            }
            line.replace_with(&lline.inner);
            Ok(())
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn run_timed_function(&mut self, id: u32) {
        if let Err(msg) = self.state.context(|ctx| -> LuaResult<()> {
            let core_table: rlua::Table = ctx.globals().get(TIMED_FUNCTION_TABLE_CORE)?;
            match core_table.get(id)? {
                rlua::Value::Function(func) => func.call::<_, ()>(()),
                _ => {
                    let table: rlua::Table = ctx.globals().get(TIMED_FUNCTION_TABLE)?;
                    match table.get(id)? {
                        rlua::Value::Function(func) => func.call::<_, ()>(()),
                        _ => Ok(()),
                    }
                }
            }
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn remove_timed_function(&mut self, id: u32) {
        if let Err(msg) = self.state.context(|ctx| -> LuaResult<()> {
            let table: rlua::Table = ctx.globals().get(TIMED_FUNCTION_TABLE)?;
            table.set(id, rlua::Nil)
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn load_script(&mut self, path: &str) -> Result<()> {
        let mut file = File::open(expand_tilde(path).as_ref())?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        if let Err(msg) = self
            .state
            .context(|ctx| -> LuaResult<()> { ctx.load(&content).set_name(path)?.exec() })
        {
            output_stack_trace(&self.writer, &msg.to_string());
        }
        Ok(())
    }

    pub fn on_connect(&mut self, host: &str, port: u16) {
        if let Err(msg) = self.state.context(|ctx| -> Result<(), rlua::Error> {
            let globals = ctx.globals();
            let table: rlua::Table = globals.get(ON_CONNECTION_CALLBACK_TABLE)?;
            for pair in table.pairs::<rlua::Value, rlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>((host, port))?;
            }
            Ok(())
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn on_disconnect(&mut self) {
        if let Err(msg) = self.state.context(|ctx| -> Result<(), rlua::Error> {
            let globals = ctx.globals();
            let table: rlua::Table = globals.get(ON_DISCONNECT_CALLBACK_TABLE)?;
            for pair in table.pairs::<rlua::Value, rlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>(())?;
            }
            Ok(())
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn set_dimensions(&mut self, dim: (u16, u16)) {
        if let Err(msg) = self.state.context(|ctx| -> LuaResult<()> {
            let mut blight: Blight = ctx.globals().get("blight")?;
            blight.screen_dimensions = dim;
            ctx.globals().set("blight", blight)?;
            Ok(())
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn proto_enabled(&mut self, proto: u8) {
        if let Err(msg) = self.state.context(|ctx| -> Result<(), rlua::Error> {
            let globals = ctx.globals();
            let table: rlua::Table = globals.get(PROTO_ENABLED_LISTENERS_TABLE)?;
            for pair in table.pairs::<rlua::Value, rlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>(proto)?;
            }
            Ok(())
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn proto_subneg(&mut self, proto: u8, bytes: &[u8]) {
        if let Err(msg) = self.state.context(|ctx| -> Result<(), rlua::Error> {
            let globals = ctx.globals();
            let table: rlua::Table = globals.get(PROTO_SUBNEG_LISTENERS_TABLE)?;
            for pair in table.pairs::<rlua::Value, rlua::Function>() {
                let (_, cb) = pair.unwrap();
                cb.call::<_, ()>((proto, bytes.to_vec()))?;
            }
            Ok(())
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
    }

    pub fn check_bindings(&mut self, cmd: &str) -> bool {
        let mut response = false;
        if let Err(msg) = self.state.context(|ctx| -> Result<(), rlua::Error> {
            let bind_table: rlua::Table = ctx.globals().get(COMMAND_BINDING_TABLE)?;
            if let Ok(callback) = bind_table.get::<_, rlua::Function>(cmd) {
                response = true;
                callback.call::<_, ()>(())
            } else {
                Ok(())
            }
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
        response
    }

    pub fn get_ui_events(&mut self) -> Vec<UiEvent> {
        match self
            .state
            .context(|ctx| -> Result<Vec<UiEvent>, rlua::Error> {
                let mut blight: Blight = ctx.globals().get("blight")?;
                let events = blight.get_ui_events();
                ctx.globals().set("blight", blight)?;
                Ok(events)
            }) {
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
    use crate::{
        event::Event,
        lua::regex::Regex,
        model::{Connection, Line},
        PROJECT_NAME, VERSION,
    };
    use rlua::Result as LuaResult;
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
        lua.state.context(|ctx| {
            ctx.load(create_trigger_lua).exec().unwrap();
        });

        assert!(test_trigger("test", &lua));
        assert!(!test_trigger("test test", &lua));
    }

    #[test]
    fn test_lua_counted_trigger() {
        let create_trigger_lua = r#"
        trigger.add("^test$", {count=3}, function () end)
        "#;

        let lua = get_lua().0;
        lua.state.context(|ctx| {
            ctx.load(create_trigger_lua).exec().unwrap();
        });

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
        lua.state.context(|ctx| {
            ctx.load(create_prompt_trigger_lua).exec().unwrap();
        });

        assert!(test_prompt_trigger("test", &lua));
        assert!(!test_prompt_trigger("test test", &lua));
    }

    #[test]
    fn test_lua_trigger_id_increment() {
        let lua = get_lua().0;
        let (ttrig, ptrig) = lua
            .state
            .context(|ctx| -> LuaResult<(u32, u32)> {
                ctx.load(r#"trigger.add("^test regular$", {}, function () end)"#)
                    .exec()
                    .unwrap();
                ctx.load(r#"trigger.add("^test regular$", {}, function () end)"#)
                    .exec()
                    .unwrap();
                let ttrig: u32 = ctx
                    .load(r#"return trigger.add("^test$", {}, function () end).id"#)
                    .call(())
                    .unwrap();
                let ptrig: u32 = ctx
                    .load(r#"return trigger.add("^test$", {prompt=true}, function () end).id"#)
                    .call(())
                    .unwrap();
                Ok((ttrig, ptrig))
            })
            .unwrap();

        assert_ne!(ttrig, ptrig);
    }

    #[test]
    fn test_lua_raw_trigger() {
        let create_trigger_lua = r#"
        trigger.add("^\\x1b\\[31mtest\\x1b\\[0m$", {raw=true}, function () end)
        "#;

        let (lua, _reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(create_trigger_lua).exec().unwrap();
        });

        assert!(test_trigger("\x1b[31mtest\x1b[0m", &lua));
        assert!(!test_trigger("test", &lua));
    }

    #[test]
    fn test_remove_trigger() {
        let lua = get_lua().0;
        let (ttrig, ptrig) = lua
            .state
            .context(|ctx| -> LuaResult<(u32, u32)> {
                let ttrig: u32 = ctx
                    .load(r#"return trigger.add("^test$", {}, function () end).id"#)
                    .call(())
                    .unwrap();
                let ptrig: u32 = ctx
                    .load(r#"return trigger.add("^test$", {prompt=true}, function () end).id"#)
                    .call(())
                    .unwrap();
                Ok((ttrig, ptrig))
            })
            .unwrap();

        assert!(test_trigger("test", &lua));
        assert!(test_prompt_trigger("test", &lua));

        lua.state.context(|ctx| {
            ctx.load(&format!("trigger.remove({})", ttrig))
                .exec()
                .unwrap();
        });

        assert!(test_prompt_trigger("test", &lua));
        assert!(!test_trigger("test", &lua));

        lua.state.context(|ctx| {
            ctx.load(&format!("trigger.remove({})", ptrig))
                .exec()
                .unwrap();
        });

        assert!(!test_trigger("test", &lua));
        assert!(!test_prompt_trigger("test", &lua));
    }

    fn check_alias_match(lua: &LuaScript, mut line: Line) -> bool {
        lua.on_mud_input(&mut line);
        line.flags.matched == true
    }

    #[test]
    fn test_lua_alias() {
        let create_alias_lua = r#"
        alias.add("^test$", function () end)
        "#;

        let lua = get_lua().0;
        lua.state.context(|ctx| {
            ctx.load(create_alias_lua).exec().unwrap();
        });

        assert!(check_alias_match(&lua, Line::from("test")));
        assert!(!check_alias_match(&lua, Line::from(" test")));
    }

    #[test]
    fn test_lua_remove_alias() {
        let create_alias_lua = r#"
        return alias.add("^test$", function () end).id
        "#;

        let lua = get_lua().0;
        let index: i32 = lua
            .state
            .context(|ctx| ctx.load(create_alias_lua).call(()))
            .unwrap();

        assert!(check_alias_match(&lua, Line::from("test")));

        let delete_alias = format!("alias.remove({})", index);
        lua.state.context(|ctx| {
            ctx.load(&delete_alias).exec().unwrap();
        });
        assert!(!check_alias_match(&lua, Line::from("test")));
    }

    #[test]
    fn test_dimensions() {
        let mut lua = get_lua().0;
        let dim: (u16, u16) = lua
            .state
            .context(|ctx| ctx.load("return blight:terminal_dimensions()").call(()))
            .unwrap();
        assert_eq!(dim, (80, 80));
        lua.set_dimensions((70, 70));
        let dim: (u16, u16) = lua
            .state
            .context(|ctx| ctx.load("return blight:terminal_dimensions()").call(()))
            .unwrap();
        assert_eq!(dim, (70, 70));
    }

    #[test]
    fn test_enable_proto() {
        let send_gmcp_lua = r#"
        core:enable_protocol(200)
        "#;

        let (lua, reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(send_gmcp_lua).exec().unwrap();
        });

        assert_eq!(reader.recv(), Ok(Event::EnableProto(200)));
    }

    #[test]
    fn test_proto_send() {
        let send_gmcp_lua = r#"
        core:subneg_send(201, { 255, 250, 86, 255, 240 })
        "#;

        let (lua, reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(send_gmcp_lua).exec().unwrap();
        });

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
            .context(|ctx| -> LuaResult<(String, String)> {
                ctx.load("return blight:version()")
                    .call::<(), (String, String)>(())
            })
            .unwrap();
        assert_eq!(version, VERSION);
        assert_eq!(name, PROJECT_NAME);
    }

    fn assert_event(lua_code: &str, event: Event) {
        let (lua, reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(lua_code).exec().unwrap();
        });

        assert_eq!(reader.recv(), Ok(event));
    }

    fn assert_events(lua_code: &str, events: Vec<Event>) {
        let (lua, reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(lua_code).exec().unwrap();
        });

        for event in events.iter() {
            assert_eq!(reader.recv(), Ok(event.clone()));
        }
    }

    #[test]
    fn test_connect() {
        assert_event(
            "blight:connect(\"hostname\", 99)",
            Event::Connect(Connection {
                host: "hostname".to_string(),
                port: 99,
                tls: false,
            }),
        );
        assert_event(
            "blight:connect(\"hostname\", 99, false)",
            Event::Connect(Connection {
                host: "hostname".to_string(),
                port: 99,
                tls: false,
            }),
        );
        assert_event(
            "blight:connect(\"hostname\", 99, true)",
            Event::Connect(Connection {
                host: "hostname".to_string(),
                port: 99,
                tls: true,
            }),
        );
    }

    #[test]
    fn test_output() {
        let (lua, _) = get_lua();
        lua.state.context(|ctx| {
            ctx.load("blight:output(\"test\", \"test\")")
                .exec()
                .unwrap();
        });
        assert_eq!(lua.get_output_lines(), vec![Line::from("test test")]);
    }

    #[test]
    fn test_load() {
        assert_event(
            "blight:load(\"/some/fancy/path\")",
            Event::LoadScript("/some/fancy/path".to_string()),
        );
    }

    #[test]
    fn test_reset() {
        assert_event("blight:reset()", Event::ResetScript);
    }

    #[test]
    fn test_sending() {
        assert_events(
            "blight:send(\"message\")",
            vec![Event::ServerInput(Line::from("message"))],
        );
    }

    #[test]
    fn test_send_bytes() {
        assert_event(
            "blight:send_bytes({ 0xff, 0xf1 })",
            Event::ServerSend(vec![0xff, 0xf1]),
        );
    }

    #[test]
    fn test_logging() {
        let (lua, reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load("blight:start_log(\"testworld\")").exec().unwrap();
            ctx.load("blight:stop_log()").exec().unwrap();
        });

        assert_eq!(
            reader.recv(),
            Ok(Event::StartLogging("testworld".to_string(), true))
        );
        assert_eq!(reader.recv(), Ok(Event::StopLogging));
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
        lua.state.context(|ctx| {
            ctx.load(trigger).exec().unwrap();
        });

        let mut line = Line::from("Health 100");
        lua.on_mud_output(&mut line);
        assert!(line.flags.gag);

        let mut line = Line::from("Health 10");
        lua.on_mud_output(&mut line);
        assert!(!line.flags.gag);
    }

    fn check_color(lua: &LuaScript, output: &str, result: &str) {
        lua.state.context(|ctx| {
            ctx.load(&format!("blight:output({})", output))
                .exec()
                .unwrap();
        });
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
        blight:bind("ctrl-a", function ()
            blight:output("ctrl-a")
        end)
        blight:bind("f1", function ()
            blight:output("f1")
        end)
        blight:bind("alt-1", function ()
            blight:output("alt-1")
        end)
        blight:bind("\x1b[1;5A", function ()
            blight:output("ctrl-up")
        end)
        "#;

        let (mut lua, _reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(lua_code).exec().unwrap();
        });

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
        blight:on_connect(function (host, port)
            blight:output(host .. ":" .. port .. "-1")
        end)
        blight:on_connect(function (host, port)
            blight:output(host .. ":" .. port .. "-2")
        end)
        blight:on_connect(function (host, port)
            blight:output(host .. ":" .. port .. "-3")
        end)
        "#;

        let (mut lua, _reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(lua_code).exec().unwrap();
        });

        lua.on_connect("test", 21);
        assert_eq!(
            lua.get_output_lines(),
            [
                Line::from("test:21-1"),
                Line::from("test:21-2"),
                Line::from("test:21-3"),
            ]
        );
        lua.reset((100, 100));
        lua.state.context(|ctx| {
            ctx.load(lua_code).exec().unwrap();
        });
        lua.on_connect("server", 1000);
        assert_eq!(
            lua.get_output_lines(),
            [
                Line::from("server:1000-1"),
                Line::from("server:1000-2"),
                Line::from("server:1000-3"),
            ]
        );
    }

    #[test]
    fn test_on_disconnect_test() {
        let lua_code = r#"
        blight:on_disconnect(function ()
            blight:output("disconnected1")
        end)
        blight:on_disconnect(function ()
            blight:output("disconnected2")
        end)
        blight:on_disconnect(function ()
            blight:output("disconnected3")
        end)
        "#;

        let (mut lua, _reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(lua_code).exec().unwrap();
        });

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
        lua.state.context(|ctx| {
            ctx.load(lua_code).exec().unwrap();
        });
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
    fn test_mud_output_command() {
        let lua_code = r#"
        trigger.add("^test trigger$", {}, function () end)
        blight:mud_output("test trigger")
        "#;

        let (lua, reader) = get_lua();
        lua.state.context(|ctx| ctx.load(lua_code).exec().unwrap());

        if let Ok(event) = reader.recv() {
            assert_eq!(event, Event::MudOutput(Line::from("test trigger")));
            if let Event::MudOutput(line) = event {
                test_trigger(&line.to_string(), &lua);
            }
        }
    }

    #[test]
    fn test_user_input_command() {
        let lua_code = r#"
        blight:user_input("test line")
        "#;

        let (lua, reader) = get_lua();
        lua.state.context(|ctx| ctx.load(lua_code).exec().unwrap());

        assert_eq!(
            reader.recv(),
            Ok(Event::ServerInput(Line::from("test line")))
        );
    }

    #[test]
    fn test_alias_ids() {
        let (lua, _reader) = get_lua();
        lua.state.context(|ctx| {
            let id = ctx
                .load(r#"return alias.add("test", function () end).id"#)
                .call(())
                .unwrap();

            let aliases: BTreeMap<u32, rlua::Table> = ctx
                .load(r#"return alias.getGroup():getAliases()"#)
                .call(())
                .unwrap();

            assert!(aliases.contains_key(&id));

            let alias: &rlua::Table = aliases.get(&id).unwrap();
            assert_eq!(alias.get::<_, bool>("enabled").unwrap(), true);
            assert_eq!(alias.get::<_, Regex>("regex").unwrap().to_string(), "test");

            ctx.load(r#"alias.clear()"#).exec().unwrap();
            let ids: BTreeMap<u32, rlua::Table> = ctx
                .load(r#"return alias.getGroup():getAliases()"#)
                .call(())
                .unwrap();

            assert!(ids.is_empty());
        });
    }

    #[test]
    fn test_trigger_ids() {
        let (lua, _reader) = get_lua();
        lua.state.context(|ctx| {
            let id = ctx
                .load(r#"return trigger.add("test", {}, function () end).id"#)
                .call(())
                .unwrap();

            let triggers: BTreeMap<u32, rlua::Table> = ctx
                .load(r#"return trigger.getGroup():getTriggers()"#)
                .call(())
                .unwrap();

            assert!(triggers.contains_key(&id));

            let trigger: &rlua::Table = triggers.get(&id).unwrap();
            assert_eq!(
                trigger.get::<_, Regex>("regex").unwrap().to_string(),
                "test"
            );
            assert_eq!(trigger.get::<_, bool>("enabled").unwrap(), true);
            assert_eq!(trigger.get::<_, bool>("gag").unwrap(), false);
            assert_eq!(trigger.get::<_, bool>("raw").unwrap(), false);
            assert_eq!(trigger.get::<_, bool>("prompt").unwrap(), false);

            ctx.load(r#"trigger.clear()"#).exec().unwrap();
            let ids: BTreeMap<u32, rlua::Table> = ctx
                .load(r#"return trigger.getGroup():getTriggers()"#)
                .call(())
                .unwrap();

            assert!(ids.is_empty());
        });
    }

    #[test]
    fn test_timer_ids() {
        let (lua, _reader) = get_lua();
        lua.state.context(|ctx| {
            ctx.load(r#"blight:clear_timers()"#).exec().unwrap();
        });
        let id = lua.state.context(|ctx| -> u32 {
            ctx.load(r#"return blight:add_timer(5, 5, function () end)"#)
                .call(())
                .unwrap()
        });

        let ids = lua.state.context(|ctx| -> Vec<u32> {
            ctx.load(r#"return blight:get_timer_ids()"#)
                .call(())
                .unwrap()
        });

        assert_eq!(ids, vec![id]);

        let ids = lua.state.context(|ctx| -> Vec<u32> {
            ctx.load(r#"blight:clear_timers()"#).exec().unwrap();
            ctx.load(r#"return blight:get_timer_ids()"#)
                .call(())
                .unwrap()
        });

        assert!(ids.is_empty());
    }

    #[test]
    fn test_remove_timer() {
        let (lua, _reader) = get_lua();

        macro_rules! timer_ids {
            () => {
                lua.state.context(|ctx| -> Vec<u32> {
                    ctx.load(r#"return blight:get_timer_ids()"#)
                        .call(())
                        .unwrap()
                })
            };
        }

        lua.state
            .context(|ctx| ctx.load(r#"blight:clear_timers()"#).exec().unwrap());

        let id1 = lua.state.context(|ctx| -> u32 {
            ctx.load(r#"return blight:add_timer(5, 5, function () end)"#)
                .call(())
                .unwrap()
        });
        let id2 = lua.state.context(|ctx| -> u32 {
            ctx.load(r#"return blight:add_timer(15, 15, function () end)"#)
                .call(())
                .unwrap()
        });

        {
            let mut active_ids = timer_ids!();
            active_ids.sort_unstable();
            let mut expected_ids = vec![id1, id2];
            expected_ids.sort_unstable();

            assert_eq!(active_ids, expected_ids);
        }

        lua.state.context(|ctx| {
            ctx.load(&format!("blight:remove_timer({})", id1))
                .exec()
                .unwrap();
        });

        assert_eq!(timer_ids!(), vec![id2]);

        let id3 = lua.state.context(|ctx| -> u32 {
            ctx.load(r#"return blight:add_timer(30, 30, function () end)"#)
                .call(())
                .unwrap()
        });

        {
            let mut active_ids = timer_ids!();
            active_ids.sort_unstable();
            let mut expected_ids = vec![id3, id2];
            expected_ids.sort_unstable();

            assert_eq!(active_ids, expected_ids);
        }

        lua.state.context(|ctx| {
            ctx.load(&format!("blight:remove_timer({})", id3))
                .exec()
                .unwrap();
        });

        assert_eq!(timer_ids!(), vec![id2]);
    }
}
