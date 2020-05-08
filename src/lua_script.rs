use crate::event::Event;
use regex::Regex;
use rlua::{Lua, Result as LuaResult, UserData, UserDataMethods, Variadic};
use std::io::prelude::*;
use std::{fs::File, result::Result, sync::mpsc::Sender};

#[derive(Clone)]
struct Alias {
    regex: Regex,
}

impl Alias {
    fn create(regex: &str) -> Result<Self, String> {
        match Regex::new(regex) {
            Ok(regex) => Ok(Self { regex }),
            Err(msg) => Err(msg.to_string()),
        }
    }
}

impl UserData for Alias {}

#[derive(Clone)]
struct RsMud {
    main_thread_writer: Sender<Event>,
}

impl RsMud {
    fn new(writer: Sender<Event>) -> Self {
        Self {
            main_thread_writer: writer,
        }
    }
}

impl UserData for RsMud {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("output", |_, this, strings: Variadic<String>| {
            this.main_thread_writer
                .send(Event::Output(strings.join(" ")))
                .unwrap();
            Ok(())
        });
        methods.add_method(
            "alias",
            |ctx, this, (regex, callback): (String, rlua::Value)| {
                if let rlua::Value::Function(func) = callback {
                    let alias_table: rlua::Table = ctx.globals().get("__alias_cb_table").unwrap();
                    match Alias::create(&regex) {
                        Ok(alias) => {
                            alias_table.set(regex.clone(), alias)?;
                            let alias_handle: rlua::AnyUserData =
                                alias_table.get(regex.clone()).unwrap();
                            alias_handle.set_user_value(func).unwrap();
                        }
                        Err(msg) => {
                            output_stack_trace(&this.main_thread_writer, &msg);
                        }
                    };
                }
                Ok(())
            },
        );
    }
}

pub struct LuaScript {
    state: Lua,
    writer: Sender<Event>,
}

impl LuaScript {
    pub fn new(main_thread_writer: Sender<Event>) -> Self {
        let state = Lua::new();

        let rsmud = RsMud::new(main_thread_writer.clone());
        state
            .context(|ctx| -> LuaResult<()> {
                let globals = ctx.globals();
                globals.set("rsmud", rsmud).unwrap();

                let alias_callback_table = ctx.create_table().unwrap();
                globals
                    .set("__alias_cb_table", alias_callback_table)
                    .unwrap();

                Ok(())
            })
            .unwrap();

        Self {
            state,
            writer: main_thread_writer,
        }
    }

    pub fn check_for_alias_match(&self, input: &str) -> bool {
        let mut response = false;
        self.state.context(|ctx| {
            let alias_table: rlua::Table = ctx.globals().get("__alias_cb_table").unwrap();
            for pair in alias_table.pairs::<String, rlua::AnyUserData>() {
                let (_, alias) = pair.unwrap();
                let regex = &alias.borrow::<Alias>().unwrap().regex;
                if regex.is_match(input) {
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

    pub fn load_script(&mut self, path: &str) {
        let mut file = File::open(path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        if let Err(msg) = self.state.context(|ctx| -> LuaResult<()> {
            ctx.load(&content).set_name(path)?.exec()?;
            Ok(())
        }) {
            output_stack_trace(&self.writer, &msg.to_string());
        }
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
