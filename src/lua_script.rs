use crate::event::Event;
use regex::Regex;
use rlua::{Lua, Result as LuaResult, UserData, UserDataMethods, Variadic};
use std::io::prelude::*;
use std::{error::Error, fs::File, result::Result, sync::mpsc::Sender};
use strip_ansi_escapes::strip as strip_ansi;

const ALIAS_TABLE: &str = "__alias_table";
const TRIGGER_TABLE: &str = "__trigger_table";
const PROMPT_TRIGGER_TABLE: &str = "__prompt_trigger_table";

#[derive(Clone)]
struct Alias {
    regex: Regex,
    enabled: bool,
}

impl Alias {
    fn create(regex: &str) -> Result<Self, String> {
        match Regex::new(regex) {
            Ok(regex) => Ok(Self {
                regex,
                enabled: true,
            }),
            Err(msg) => Err(msg.to_string()),
        }
    }
}

impl UserData for Alias {}

#[derive(Clone)]
struct Trigger {
    regex: Regex,
    gag: bool,
    enabled: bool,
}

impl Trigger {
    fn create(regex: &str) -> Result<Self, String> {
        match Regex::new(regex) {
            Ok(regex) => Ok(Self {
                regex,
                gag: false,
                enabled: true,
            }),
            Err(msg) => Err(msg.to_string()),
        }
    }
}

impl UserData for Trigger {}

#[derive(Clone)]
struct BlightMud {
    main_thread_writer: Sender<Event>,
}

impl BlightMud {
    fn new(writer: Sender<Event>) -> Self {
        Self {
            main_thread_writer: writer,
        }
    }

    fn create_trigger(&self, regex: &str, gag: bool) -> Result<Trigger, String> {
        match Trigger::create(&regex) {
            Ok(mut trigger) => {
                trigger.gag = gag;
                Ok(trigger)
            }
            Err(msg) => Err(format!("Failed to parse regex: {}", &msg)),
        }
    }
}

impl UserData for BlightMud {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("connect", |_, this, (host, port): (String, u32)| {
            this.main_thread_writer
                .send(Event::Connect(host, port))
                .unwrap();
            Ok(())
        });
        methods.add_method("reset", |_, this, ()| {
            this.main_thread_writer.send(Event::ResetScript).unwrap();
            Ok(())
        });
        methods.add_method("load", |_, this, path: String| {
            this.main_thread_writer
                .send(Event::LoadScript(path))
                .unwrap();
            Ok(())
        });
        methods.add_method("output", |_, this, strings: Variadic<String>| {
            this.main_thread_writer
                .send(Event::Output(strings.join(" ")))
                .unwrap();
            Ok(())
        });
        methods.add_method("send", |_, this, strings: Variadic<String>| {
            this.main_thread_writer
                .send(Event::ServerInput(strings.join(" "), false))
                .unwrap();
            Ok(())
        });
        methods.add_method(
            "add_alias",
            |ctx, this, (regex, callback): (String, rlua::Function)| {
                let alias_table: rlua::Table = ctx.globals().get(ALIAS_TABLE)?;
                let next_index = alias_table.raw_len() + 1;
                match Alias::create(&regex) {
                    Ok(alias) => {
                        alias_table.set(next_index, alias)?;
                        let alias_handle: rlua::AnyUserData = alias_table.get(next_index)?;
                        alias_handle.set_user_value(callback)?;
                    }
                    Err(msg) => {
                        output_stack_trace(&this.main_thread_writer, &msg);
                    }
                };
                Ok(next_index)
            },
        );
        methods.add_method(
            "add_trigger",
            |ctx, this, (regex, options, callback): (String, rlua::Table, rlua::Function)| {
                let trigger_table: rlua::Table = ctx.globals().get(TRIGGER_TABLE)?;
                let next_index = trigger_table.raw_len() + 1;
                match this.create_trigger(&regex, false) {
                    Ok(mut trigger) => {
                        trigger.gag = options.get("gag")?;
                        trigger_table.set(next_index, trigger)?;
                        let trigger_handle: rlua::AnyUserData = trigger_table.get(next_index)?;
                        trigger_handle.set_user_value(callback)?;
                    }
                    Err(msg) => {
                        output_stack_trace(&this.main_thread_writer, &msg);
                    }
                }
                Ok(next_index)
            },
        );
        methods.add_method(
            "add_prompt_trigger",
            |ctx, this, (regex, options, callback): (String, rlua::Table, rlua::Function)| {
                let trigger_table: rlua::Table = ctx.globals().get(PROMPT_TRIGGER_TABLE)?;
                let next_index = trigger_table.raw_len() + 1;
                match Trigger::create(&regex) {
                    Ok(mut trigger) => {
                        trigger.gag = options.get("gag")?;
                        trigger_table.set(next_index, trigger)?;
                        let trigger_handle: rlua::AnyUserData = trigger_table.get(next_index)?;
                        trigger_handle.set_user_value(callback)?;
                    }
                    Err(msg) => {
                        output_stack_trace(&this.main_thread_writer, &msg);
                    }
                };
                Ok(next_index)
            },
        );
    }
}

pub struct LuaScript {
    state: Lua,
    writer: Sender<Event>,
}

fn create_default_lua_state(writer: Sender<Event>) -> Lua {
    let state = Lua::new();

    let blight = BlightMud::new(writer);
    state
        .context(|ctx| -> LuaResult<()> {
            let globals = ctx.globals();
            globals.set("blight", blight).unwrap();

            let alias_table = ctx.create_table().unwrap();
            globals.set(ALIAS_TABLE, alias_table).unwrap();
            let trigger_table = ctx.create_table().unwrap();
            globals.set(TRIGGER_TABLE, trigger_table).unwrap();
            let prompt_trigger = ctx.create_table().unwrap();
            globals.set(PROMPT_TRIGGER_TABLE, prompt_trigger).unwrap();

            Ok(())
        })
        .unwrap();
    state
}

impl LuaScript {
    pub fn new(main_thread_writer: Sender<Event>) -> Self {
        Self {
            state: create_default_lua_state(main_thread_writer.clone()),
            writer: main_thread_writer,
        }
    }

    pub fn reset(&mut self) {
        self.state = create_default_lua_state(self.writer.clone());
    }

    pub fn check_for_alias_match(&self, input: &str) -> bool {
        let mut response = false;
        self.state.context(|ctx| {
            let alias_table: rlua::Table = ctx.globals().get(ALIAS_TABLE).unwrap();
            for pair in alias_table.pairs::<rlua::Value, rlua::AnyUserData>() {
                let (_, alias) = pair.unwrap();
                let rust_alias = &alias.borrow::<Alias>().unwrap();
                let regex = &rust_alias.regex;
                if rust_alias.enabled && regex.is_match(input) {
                    let cb: rlua::Function = alias.get_user_value().unwrap();
                    let captures: Vec<String> = regex
                        .captures(input)
                        .unwrap()
                        .iter()
                        .map(|c| match c {
                            Some(m) => m.as_str().to_string(),
                            None => String::new(),
                        })
                        .collect();
                    if let Err(msg) = cb.call::<_, ()>(captures) {
                        output_stack_trace(&self.writer, &msg.to_string());
                    }
                    response = true;
                }
            }
        });
        response
    }

    pub fn check_for_trigger_match(&self, input: &str) -> bool {
        self.check_trigger_match(input, TRIGGER_TABLE)
    }

    pub fn check_for_prompt_trigger_match(&self, input: &str) -> bool {
        self.check_trigger_match(input, PROMPT_TRIGGER_TABLE)
    }

    fn check_trigger_match(&self, input: &str, table: &str) -> bool {
        let clean_bytes = strip_ansi(input.as_bytes()).unwrap();
        let input = &String::from_utf8_lossy(&clean_bytes);
        let mut response = false;
        self.state.context(|ctx| {
            let trigger_table: rlua::Table = ctx.globals().get(table).unwrap();
            for pair in trigger_table.pairs::<rlua::Value, rlua::AnyUserData>() {
                let (_, trigger) = pair.unwrap();
                let rust_trigger = &trigger.borrow::<Trigger>().unwrap();
                if rust_trigger.enabled && rust_trigger.regex.is_match(input) {
                    let cb: rlua::Function = trigger.get_user_value().unwrap();
                    let captures: Vec<String> = rust_trigger
                        .regex
                        .captures(input)
                        .unwrap()
                        .iter()
                        .map(|c| match c {
                            Some(m) => m.as_str().to_string(),
                            None => String::new(),
                        })
                        .collect();
                    if let Err(msg) = cb.call::<_, ()>(captures) {
                        output_stack_trace(&self.writer, &msg.to_string());
                    }
                    response = rust_trigger.gag;
                }
            }
        });
        response
    }

    pub fn load_script(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut file = File::open(path)?;
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
}

fn output_stack_trace(writer: &Sender<Event>, error: &str) {
    writer
        .send(Event::Error("[Lua] Script error:".to_string()))
        .unwrap();
    for line in error.split('\n') {
        writer
            .send(Event::Error(format!("\t{}", line).to_string()))
            .unwrap();
    }
}
