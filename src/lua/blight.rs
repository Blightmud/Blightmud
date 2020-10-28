use super::{
    alias::Alias, constants::*, store_data::StoreData, trigger::Trigger, ui_event::UiEvent,
    util::output_stack_trace,
};
use crate::event::Event;
use crate::{
    io::SaveData,
    model::{Connection, Line},
    PROJECT_NAME, VERSION,
};
use anyhow::Result;
use chrono::Duration;
use log::debug;
use rlua::{Result as LuaResult, UserData, UserDataMethods, Variadic};
use std::{collections::BTreeMap, sync::mpsc::Sender};

#[derive(Clone)]
pub struct Blight {
    main_writer: Sender<Event>,
    output_lines: Vec<Line>,
    ui_events: Vec<UiEvent>,
    pub screen_dimensions: (u16, u16),
    next_id: u32,
    core_mode: bool,
}

impl Blight {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            output_lines: vec![],
            ui_events: vec![],
            screen_dimensions: (0, 0),
            next_id: 0,
            core_mode: false,
        }
    }

    fn next_index(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    pub fn core_mode(&mut self, mode: bool) {
        self.core_mode = mode;
    }

    fn alias_table(&self) -> &'static str {
        if self.core_mode {
            ALIAS_TABLE_CORE
        } else {
            ALIAS_TABLE
        }
    }

    fn trigger_table(&self) -> &'static str {
        if self.core_mode {
            TRIGGER_TABLE_CORE
        } else {
            TRIGGER_TABLE
        }
    }

    fn timer_table(&self) -> &'static str {
        TIMED_FUNCTION_TABLE
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

    fn create_trigger(&self, regex: &str) -> Result<Trigger, String> {
        match Trigger::create(&regex) {
            Ok(trigger) => Ok(trigger),
            Err(msg) => Err(format!("Failed to parse regex: {}", &msg)),
        }
    }
}

impl UserData for Blight {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("terminal_dimensions", |_, this, _: ()| {
            Ok(this.screen_dimensions)
        });
        methods.add_method(
            "connect",
            |_, this, (host, port, tls): (String, u16, Option<bool>)| {
                this.main_writer
                    .send(Event::Connect(Connection { host, port, tls }))
                    .unwrap();
                Ok(())
            },
        );
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
        methods.add_method("mud_output", |_, this, msg: String| {
            this.main_writer
                .send(Event::MudOutput(Line::from(msg)))
                .unwrap();
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
        methods.add_method("user_input", |_, this, line: String| {
            this.main_writer
                .send(Event::ServerInput(Line::from(line)))
                .unwrap();
            Ok(())
        });
        methods.add_method("debug", |_, _, strings: Variadic<String>| {
            debug!("{}", strings.join(" "));
            Ok(())
        });
        methods.add_method(
            "store",
            |_, _, (id, data): (String, rlua::Value)| -> LuaResult<()> {
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

                if let Ok(mut store_data) = StoreData::load() {
                    store_data.insert(id, data);
                    store_data.save().unwrap();
                    Ok(())
                } else {
                    Err(rlua::Error::RuntimeError(
                        "Failed to access store file".to_string(),
                    ))
                }
            },
        );
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
        methods.add_method_mut(
            "add_alias",
            |ctx, this, (regex, callback): (String, rlua::Function)| {
                let alias_table: rlua::Table = ctx.globals().get(this.alias_table())?;
                let next_index = this.next_index();
                match Alias::create(&regex) {
                    Ok(alias) => {
                        alias_table.raw_set(next_index, alias)?;
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
        methods.add_method("enable_alias", |ctx, this, (id, enabled): (i32, bool)| {
            let table: rlua::Table = ctx.globals().raw_get(this.alias_table())?;

            // Retrieve the callback function
            let cb: rlua::Function = { table.get::<i32, rlua::AnyUserData>(id)?.get_user_value()? };
            let mut alias: Alias = table.get(id)?;
            alias.enabled = enabled;
            table.raw_set(id, alias)?;

            // Reset the callback function
            let alias_handle: rlua::AnyUserData = table.get(id)?;
            alias_handle.set_user_value(cb)?;

            Ok(())
        });
        methods.add_method("remove_alias", |ctx, this, alias_idx: i32| {
            let alias_table: rlua::Table = ctx.globals().get(this.alias_table())?;
            alias_table.raw_set(alias_idx, rlua::Nil)
        });
        methods.add_method("get_aliases", |ctx, this, ()| {
            let alias_table: rlua::Table = ctx.globals().get(this.alias_table())?;
            let mut keys: BTreeMap<rlua::Integer, Alias> = BTreeMap::new();
            for pair in alias_table.pairs::<rlua::Integer, Alias>() {
                let (id, alias) = pair?;
                keys.insert(id, alias);
            }
            Ok(keys)
        });
        methods.add_method("clear_aliases", |ctx, this, ()| {
            ctx.globals().set(this.alias_table(), ctx.create_table()?)?;
            Ok(())
        });
        methods.add_method_mut(
            "add_trigger",
            |ctx, this, (regex, options, callback): (String, rlua::Table, rlua::Function)| {
                let next_index = this.next_index();

                let trigger_table: rlua::Table = if !options.get("prompt")? {
                    ctx.globals().get(this.trigger_table())?
                } else {
                    ctx.globals().get(PROMPT_TRIGGER_TABLE)?
                };

                match this.create_trigger(&regex) {
                    Ok(mut trigger) => {
                        trigger.gag = options.get("gag")?;
                        trigger.raw = options.get("raw")?;
                        trigger.prompt = options.get("prompt")?;
                        trigger.count = options.get("count").ok().unwrap_or_default();
                        trigger.enabled = !options.get("enabled")?;
                        trigger_table.raw_set(next_index, trigger)?;
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
        methods.add_method("enable_trigger", |ctx, this, (id, enabled): (i32, bool)| {
            let table: rlua::Table = ctx.globals().raw_get(this.trigger_table())?;

            // Retrieve the callback function
            let cb: rlua::Function = { table.get::<i32, rlua::AnyUserData>(id)?.get_user_value()? };
            let mut trigger: Trigger = table.get(id)?;
            trigger.enabled = enabled;
            table.raw_set(id, trigger)?;

            // Reset the callback function
            let trigger_handle: rlua::AnyUserData = table.get(id)?;
            trigger_handle.set_user_value(cb)?;

            Ok(())
        });
        methods.add_method("remove_trigger", |ctx, this, trigger_idx: i32| {
            let trigger_table: rlua::Table = {
                let triggers: rlua::Table = ctx.globals().get(this.trigger_table())?;
                let prompts: rlua::Table = ctx.globals().get(PROMPT_TRIGGER_TABLE)?;
                if triggers.contains_key(trigger_idx)? {
                    triggers
                } else {
                    prompts
                }
            };
            trigger_table.set(trigger_idx, rlua::Nil)
        });
        methods.add_method("get_triggers", |ctx, this, ()| {
            let trigger_table: rlua::Table = ctx.globals().get(this.trigger_table())?;
            let prompt_trigger_table: rlua::Table = ctx.globals().get(PROMPT_TRIGGER_TABLE)?;
            let mut triggers: BTreeMap<rlua::Integer, Trigger> = BTreeMap::new();
            let trigger_it = trigger_table.pairs::<rlua::Integer, Trigger>();
            let prompt_it = prompt_trigger_table.pairs::<rlua::Integer, Trigger>();
            for pair in trigger_it.chain(prompt_it) {
                let (id, trigger) = pair?;
                triggers.insert(id, trigger);
            }
            Ok(triggers)
        });
        methods.add_method("clear_triggers", |ctx, this, ()| {
            ctx.globals()
                .set(this.trigger_table(), ctx.create_table()?)?;
            ctx.globals()
                .set(PROMPT_TRIGGER_TABLE, ctx.create_table()?)?;
            Ok(())
        });
        methods.add_method("gag", |ctx, _, _: ()| {
            ctx.globals().set(GAG_NEXT_TRIGGER_LINE, true)
        });
        methods.add_method_mut(
            "add_timer",
            |ctx, this, (duration, count, callback): (f32, u32, rlua::Function)| {
                let duration = Duration::milliseconds((duration * 1000.0) as i64);
                let count = if count > 0 { Some(count) } else { None };
                let cb_table: rlua::Table = ctx.globals().get(this.timer_table())?;
                let next_index = this.next_index();
                cb_table.raw_set(next_index, callback)?;
                this.main_writer
                    .send(Event::AddTimedEvent(duration, count, next_index as u32))
                    .unwrap();
                Ok(next_index)
            },
        );
        methods.add_method("get_timer_ids", |ctx, this, ()| {
            let timer_table: rlua::Table = ctx.globals().get(this.timer_table())?;
            let mut keys: Vec<rlua::Integer> = vec![];
            for pair in timer_table.pairs::<rlua::Integer, rlua::Value>() {
                keys.push(pair?.0);
            }
            Ok(keys)
        });
        methods.add_method("clear_timers", |ctx, this, ()| {
            this.main_writer.send(Event::ClearTimers).unwrap();
            ctx.globals().set(this.timer_table(), ctx.create_table()?)?;
            Ok(())
        });
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
        methods.add_method("on_connect", |ctx, _, callback: rlua::Function| {
            let globals = ctx.globals();
            let table: rlua::Table = globals.get(ON_CONNECTION_CALLBACK_TABLE)?;
            table.set(table.raw_len() + 1, callback)?;
            globals.set(ON_CONNECTION_CALLBACK_TABLE, table)?;
            Ok(())
        });
        methods.add_method("on_disconnect", |ctx, _, callback: rlua::Function| {
            let globals = ctx.globals();
            let table: rlua::Table = globals.get(ON_DISCONNECT_CALLBACK_TABLE)?;
            table.set(table.raw_len() + 1, callback)?;
            globals.set(ON_DISCONNECT_CALLBACK_TABLE, table)?;
            Ok(())
        });
        methods.add_method("version", |_, _, _: ()| -> LuaResult<(&str, &str)> {
            Ok((PROJECT_NAME, VERSION))
        });
    }
}

#[cfg(test)]
mod user_data_tests {

    use super::Blight;
    use crate::{event::Event, lua::constants::*};
    use std::sync::mpsc::{channel, Receiver, Sender};

    fn get_blight() -> (Blight, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        (Blight::new(writer), reader)
    }

    #[test]
    fn confirm_core_mode() {
        let (mut blight, _reader) = get_blight();
        blight.core_mode(true);
        assert_eq!(blight.alias_table(), ALIAS_TABLE_CORE);
        assert_eq!(blight.trigger_table(), TRIGGER_TABLE_CORE);
        assert_eq!(blight.timer_table(), TIMED_FUNCTION_TABLE);
        blight.core_mode(false);
        assert_eq!(blight.alias_table(), ALIAS_TABLE);
        assert_eq!(blight.trigger_table(), TRIGGER_TABLE);
        assert_eq!(blight.timer_table(), TIMED_FUNCTION_TABLE);
    }

    #[test]
    fn test_next_id() {
        let (mut blight, _reader) = get_blight();
        assert_eq!(blight.next_id, 0);
        assert_eq!(blight.next_index(), 1);
        assert_eq!(blight.next_id, 1);
        assert_eq!(blight.next_index(), 2);
    }
}
