use crate::event::Event;
use rlua::{Lua, Result, UserData, UserDataMethods, Variadic};
use std::io::prelude::*;
use std::{fs::File, sync::mpsc::Sender};

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
        methods.add_method_mut("output", |_, this, strings: Variadic<String>| {
            this.main_thread_writer
                .send(Event::Output(strings.join(" ")))
                .unwrap();
            Ok(())
        });
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
            .context(|ctx| -> Result<()> {
                let globals = ctx.globals();
                globals.set("rsmud", rsmud).unwrap();

                Ok(())
            })
            .unwrap();

        Self {
            state,
            writer: main_thread_writer,
        }
    }

    pub fn load_script(&mut self, path: &str) {
        let mut file = File::open(path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        if let Err(msg) = self.state.context(|ctx| -> Result<()> {
            ctx.load(&content).set_name(path)?.exec()?;
            Ok(())
        }) {
            self.writer
                .send(Event::Error("[Lua] Script error:".to_string()))
                .unwrap();
            for line in msg.to_string().split('\n') {
                self.writer
                    .send(Event::Error(format!("\t{}", line).to_string()))
                    .unwrap();
            }
        }
    }
}
