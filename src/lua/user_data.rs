use super::{constants::*, store_data::StoreData, util::output_stack_trace, UiEvent};
use crate::event::Event;
use crate::{
    io::SaveData,
    model::{Connection, Line},
    PROJECT_NAME, VERSION,
};
use anyhow::Result;
use chrono::Duration;
use log::debug;
use regex::Regex;
use rlua::{Result as LuaResult, UserData, UserDataMethods, Variadic};
use std::{collections::BTreeMap, sync::mpsc::Sender};

fn cursor_event_from_str(event: &str) -> Option<UiEvent> {
    match event {
        "step_left" => Some(UiEvent::StepLeft),
        "step_right" => Some(UiEvent::StepRight),
        "step_to_start" => Some(UiEvent::StepToStart),
        "step_to_end" => Some(UiEvent::StepToEnd),
        "step_word_left" => Some(UiEvent::StepWordLeft),
        "step_word_right" => Some(UiEvent::StepWordRight),
        "delete" => Some(UiEvent::Remove),
        "delete_to_end" => Some(UiEvent::DeleteToEnd),
        "delete_from_start" => Some(UiEvent::DeleteFromStart),
        "previous_command" => Some(UiEvent::PreviousCommand),
        "next_command" => Some(UiEvent::NextCommand),
        "scroll_up" => Some(UiEvent::ScrollUp),
        "scroll_down" => Some(UiEvent::ScrollDown),
        "scroll_bottom" => Some(UiEvent::ScrollBottom),
        "complete" => Some(UiEvent::Complete),
        _ => None,
    }
}

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
    pub raw: bool,
    pub enabled: bool,
}

impl Trigger {
    pub fn create(regex: &str) -> Result<Self, String> {
        match Regex::new(regex) {
            Ok(regex) => Ok(Self {
                regex,
                gag: false,
                raw: false,
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
    ui_events: Vec<UiEvent>,
    pub screen_dimensions: (u16, u16),
}

impl BlightMud {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            output_lines: vec![],
            ui_events: vec![],
            screen_dimensions: (0, 0),
        }
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
        methods.add_method("terminal_dimensions", |_, this, _: ()| {
            Ok(this.screen_dimensions)
        });
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
            "bind",
            |ctx, _, (cmd, callback): (String, rlua::Function)| {
                let bind_table: rlua::Table = ctx.globals().get(COMMAND_BINDING_TABLE)?;
                bind_table.set(cmd.to_lowercase(), callback)?;
                Ok(())
            },
        );
        methods.add_method("unbind", |ctx, _, cmd: String| {
            let bind_table: rlua::Table = ctx.globals().get(COMMAND_BINDING_TABLE)?;
            bind_table.set(cmd, rlua::Nil)?;
            Ok(())
        });
        methods.add_method_mut("ui", |_, this, cmd: String| {
            if let Some(cmd) = cursor_event_from_str(&cmd) {
                this.ui_events.push(cmd);
            } else {
                this.main_writer
                    .send(Event::Error(format!("Invalid ui command: {}", cmd)))
                    .unwrap();
            }
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
        methods.add_method("send_bytes", |_, this, bytes: Vec<u8>| {
            this.main_writer.send(Event::ServerSend(bytes)).unwrap();
            Ok(())
        });
        methods.add_method("debug", |_, _, strings: Variadic<String>| {
            debug!("{}", strings.join(" "));
            Ok(())
        });
        methods.add_method("store", |_, _, (id, data): (String, rlua::Value)| {
            let data = match data {
                rlua::Value::Table(table) => {
                    let mut map: BTreeMap<String, String> = BTreeMap::new();
                    let iter = table.pairs();
                    for entry in iter {
                        if let Ok((key, value)) = entry {
                            map.insert(key, value);
                        }
                    }
                    Ok(map)
                }
                _ => Err(rlua::Error::RuntimeError(
                    "Bad data! You may only store tables".to_string(),
                )),
            }?;

            let mut store_data = StoreData::load().unwrap();
            store_data.insert(id, data);

            store_data.save().unwrap();
            Ok(())
        });
        methods.add_method(
            "read",
            |_, _, id: String| -> LuaResult<Option<BTreeMap<String, String>>> {
                let data = StoreData::load().unwrap();
                Ok(match data.get(&id) {
                    Some(data) => Some(data.clone()),
                    _ => None,
                })
            },
        );
        methods.add_method("start_log", |_, this, name: String| {
            this.main_writer
                .send(Event::StartLogging(name, true))
                .unwrap();
            Ok(())
        });
        methods.add_method("stop_log", |_, this, _: ()| {
            this.main_writer.send(Event::StopLogging).unwrap();
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
                        trigger.raw = options.get("raw")?;
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
        methods.add_method("gag", |ctx, _, _: ()| {
            ctx.globals().set(GAG_NEXT_TRIGGER_LINE, true)
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
        methods.add_method("status_height", |_, this, height: u16| {
            this.main_writer
                .send(Event::StatusAreaHeight(height))
                .unwrap();
            Ok(())
        });
        methods.add_method("status_line", |_, this, (index, line): (usize, String)| {
            this.main_writer
                .send(Event::StatusLine(index, line))
                .unwrap();
            Ok(())
        });
        methods.add_method("send_gmcp", |_, this, msg: String| {
            this.main_writer.send(Event::GMCPSend(msg)).unwrap();
            Ok(())
        });
        methods.add_method("on_connect", |ctx, _, callback: rlua::Function| {
            ctx.globals().set(ON_CONNCTION_CALLBACK, callback)
        });
        methods.add_method("on_disconnect", |ctx, _, callback: rlua::Function| {
            ctx.globals().set(ON_DISCONNECT_CALLBACK, callback)
        });
        methods.add_method("on_gmcp_ready", |ctx, _, callback: rlua::Function| {
            ctx.globals().set(ON_GMCP_READY_CALLBACK, callback)
        });
        methods.add_method("version", |_, _, _: ()| -> LuaResult<(&str, &str)> {
            Ok((PROJECT_NAME, VERSION))
        });
    }
}
