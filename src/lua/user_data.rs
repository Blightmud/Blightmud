use super::{constants::*, util::output_stack_trace};
use crate::connection::Connection;
use crate::event::Event;
use chrono::Duration;
use log::debug;
use regex::Regex;
use rlua::{UserData, UserDataMethods, Variadic};
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
    main_thread_writer: Sender<Event>,
}

impl BlightMud {
    pub fn new(writer: Sender<Event>) -> Self {
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
        methods.add_method("connect", |_, this, (host, port): (String, u16)| {
            this.main_thread_writer
                .send(Event::Connect(Connection { host, port }))
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
                        output_stack_trace(&this.main_thread_writer, &msg);
                    }
                };
                Ok(next_index)
            },
        );
        methods.add_method(
            "add_trigger",
            |ctx, this, (regex, options, callback): (String, rlua::Table, rlua::Function)| {
                let trigger_table: rlua::Table = if options.contains_key("prompt")? {
                    ctx.globals().get(PROMPT_TRIGGER_TABLE)?
                } else {
                    ctx.globals().get(TRIGGER_TABLE)?
                };
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
            "add_timer",
            |ctx, this, (duration, count, callback): (f32, u32, rlua::Function)| {
                let duration = Duration::milliseconds((duration * 1000.0) as i64);
                let count = if count > 0 { Some(count) } else { None };
                let cb_table: rlua::Table = ctx.globals().get(TIMED_FUNCTION_TABLE)?;
                let next_index = cb_table.raw_len() + 1;
                cb_table.set(next_index, callback)?;
                this.main_thread_writer
                    .send(Event::AddTimedEvent(duration, count, next_index as u32))
                    .unwrap();
                Ok(next_index)
            },
        );
        methods.add_method("register_gmcp", |_, this, module: String| {
            this.main_thread_writer
                .send(Event::GMCPRegister(module))
                .unwrap();
            Ok(())
        });
        methods.add_method(
            "add_gmcp_receiver",
            |ctx, _, (msg_type, callback): (String, rlua::Function)| {
                let gmcp_table: rlua::Table = ctx.globals().get(GMCP_LISTENER_TABLE)?;
                gmcp_table.set(msg_type, callback).unwrap();
                Ok(())
            },
        );
        methods.add_method("on_connect", |ctx, _, callback: rlua::Function| {
            ctx.globals().set(ON_CONNCTION_CALLBACK, callback)?;
            Ok(())
        });
        methods.add_method("on_gmcp_ready", |ctx, _, callback: rlua::Function| {
            ctx.globals().set(ON_GMCP_READY_CALLBACK, callback)?;
            Ok(())
        });
    }
}
