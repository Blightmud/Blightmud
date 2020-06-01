use super::{constants::*, util::output_stack_trace};
use crate::event::Event;
use crate::{
    model::{Connection, Line},
    PROJECT_NAME, VERSION,
};
use anyhow::Result;
use chrono::Duration;
use log::debug;
use regex::Regex;
use rlua::{Result as LuaResult, UserData, UserDataMethods, Variadic};
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Alias {
    pub regex: Regex,
    pub enabled: bool,
}

impl Alias {
    pub fn create(regex: &str) -> Result<Self, String> {
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
pub struct Trigger {
    pub regex: Regex,
    pub gag: bool,
    pub enabled: bool,
}

impl Trigger {
    pub fn create(regex: &str) -> Result<Self, String> {
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
pub struct BlightMud {
    main_writer: Sender<Event>,
    output_lines: Vec<Line>,
}

impl BlightMud {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            output_lines: vec![],
        }
    }

    pub fn get_output_lines(&mut self) -> Vec<Line> {
        let return_lines = self.output_lines.clone();
        self.output_lines.clear();
        return_lines
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
        methods.add_method("connect", |_, this, (host, port): (String, u16)| {
            this.main_writer
                .send(Event::Connect(Connection { host, port }))
                .unwrap();
            Ok(())
        });
        methods.add_method("reset", |_, this, ()| {
            this.main_writer.send(Event::ResetScript).unwrap();
            Ok(())
        });
        methods.add_method("load", |_, this, path: String| {
            this.main_writer.send(Event::LoadScript(path)).unwrap();
            Ok(())
        });
        methods.add_method_mut("output", |_, this, strings: Variadic<String>| {
            this.output_lines.push(Line::from(strings.join(" ")));
            Ok(())
        });
        methods.add_method(
            "send",
            |_, this, (msg, options): (String, Option<rlua::Table>)| {
                let mut line = Line::from(msg);
                line.flags.bypass_script = true;

                if let Some(table) = options {
                    line.flags.gag = table.get("gag")?;
                    line.flags.skip_log = table.get("skip_log")?;
                }

                this.main_writer.send(Event::ServerInput(line)).unwrap();
                Ok(())
            },
        );
        methods.add_method("debug", |_, _, strings: Variadic<String>| {
            debug!("{}", strings.join(" "));
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
                        output_stack_trace(&this.main_writer, &msg);
                    }
                };
                Ok(next_index)
            },
        );
        methods.add_method("remove_alias", |ctx, _, alias_idx: i32| {
            let alias_table: rlua::Table = ctx.globals().get(ALIAS_TABLE)?;
            alias_table.set(alias_idx, rlua::Nil)
        });
        methods.add_method(
            "add_trigger",
            |ctx, this, (regex, options, callback): (String, rlua::Table, rlua::Function)| {
                let trigger_table: rlua::Table = if options.contains_key("prompt")? {
                    ctx.globals().get(PROMPT_TRIGGER_TABLE)?
                } else {
                    ctx.globals().get(TRIGGER_TABLE)?
                };

                let next_index = {
                    let triggers: rlua::Table = ctx.globals().get(TRIGGER_TABLE)?;
                    let prompts: rlua::Table = ctx.globals().get(PROMPT_TRIGGER_TABLE)?;
                    prompts.raw_len().max(triggers.raw_len()) + 1
                };

                match this.create_trigger(&regex, false) {
                    Ok(mut trigger) => {
                        trigger.gag = options.get("gag")?;
                        trigger_table.set(next_index, trigger)?;
                        let trigger_handle: rlua::AnyUserData = trigger_table.get(next_index)?;
                        trigger_handle.set_user_value(callback)?;
                    }
                    Err(msg) => {
                        output_stack_trace(&this.main_writer, &msg);
                    }
                }
                Ok(next_index)
            },
        );
        methods.add_method("remove_trigger", |ctx, _, trigger_idx: i32| {
            let trigger_table: rlua::Table = {
                let triggers: rlua::Table = ctx.globals().get(TRIGGER_TABLE)?;
                let prompts: rlua::Table = ctx.globals().get(PROMPT_TRIGGER_TABLE)?;
                if triggers.contains_key(trigger_idx)? {
                    triggers
                } else {
                    prompts
                }
            };
            trigger_table.set(trigger_idx, rlua::Nil)
        });
        methods.add_method(
            "add_timer",
            |ctx, this, (duration, count, callback): (f32, u32, rlua::Function)| {
                let duration = Duration::milliseconds((duration * 1000.0) as i64);
                let count = if count > 0 { Some(count) } else { None };
                let cb_table: rlua::Table = ctx.globals().get(TIMED_FUNCTION_TABLE)?;
                let next_index = cb_table.raw_len() + 1;
                cb_table.set(next_index, callback)?;
                this.main_writer
                    .send(Event::AddTimedEvent(duration, count, next_index as u32))
                    .unwrap();
                Ok(next_index)
            },
        );
        methods.add_method("register_gmcp", |_, this, module: String| {
            this.main_writer.send(Event::GMCPRegister(module)).unwrap();
            Ok(())
        });
        methods.add_method(
            "add_gmcp_receiver",
            |ctx, _, (msg_type, callback): (String, rlua::Function)| {
                let gmcp_table: rlua::Table = ctx.globals().get(GMCP_LISTENER_TABLE)?;
                gmcp_table.set(msg_type, callback)?;
                Ok(())
            },
        );
        methods.add_method("send_gmcp", |_, this, msg: String| {
            this.main_writer.send(Event::GMCPSend(msg)).unwrap();
            Ok(())
        });
        methods.add_method("on_connect", |ctx, _, callback: rlua::Function| {
            ctx.globals().set(ON_CONNCTION_CALLBACK, callback)?;
            Ok(())
        });
        methods.add_method("on_gmcp_ready", |ctx, _, callback: rlua::Function| {
            ctx.globals().set(ON_GMCP_READY_CALLBACK, callback)?;
            Ok(())
        });
        methods.add_method("version", |_, _, _: ()| -> LuaResult<(&str, &str)> {
            Ok((PROJECT_NAME, VERSION))
        });
    }
}
