use super::{constants::*, store_data::StoreData, ui_event::UiEvent};
use crate::event::Event;
use crate::{io::SaveData, model::Line, PROJECT_NAME, VERSION};
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

    fn timer_table(&self) -> &'static str {
        if self.core_mode {
            TIMED_FUNCTION_TABLE_CORE
        } else {
            TIMED_FUNCTION_TABLE
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
}

impl UserData for Blight {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("output", |_, this, strings: Variadic<String>| {
            this.output_lines.push(Line::from(strings.join(" ")));
            Ok(())
        });
        methods.add_method("terminal_dimensions", |_, this, _: ()| {
            Ok(this.screen_dimensions)
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

                let mut store_data = StoreData::load();
                store_data.insert(id, data);
                store_data.save();
                Ok(())
            },
        );
        methods.add_method(
            "read",
            |_, _, id: String| -> LuaResult<Option<BTreeMap<String, String>>> {
                let data = StoreData::load();
                Ok(match data.get(&id) {
                    Some(data) => Some(data.clone()),
                    _ => None,
                })
            },
        );
        methods.add_method("is_core_mode", |_, this, ()| Ok(this.core_mode));
        methods.add_method_mut(
            "add_timer",
            |ctx, this, (duration, count, callback): (f32, u32, rlua::Function)| {
                let duration = Duration::milliseconds((duration * 1000.0) as i64);
                let count = if count > 0 { Some(count) } else { None };
                let cb_table: rlua::Table = ctx.globals().get(this.timer_table())?;
                let next_index = this.next_index();
                cb_table.raw_set(next_index, callback)?;
                this.main_writer
                    .send(Event::AddTimedEvent(
                        duration,
                        count,
                        next_index as u32,
                        this.core_mode,
                    ))
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
        methods.add_method("remove_timer", |ctx, this, timer_idx: u32| {
            this.main_writer
                .send(Event::RemoveTimer(timer_idx))
                .unwrap();
            let timer_table: rlua::Table = ctx.globals().get(this.timer_table())?;
            timer_table.set(timer_idx, rlua::Nil)
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
        assert_eq!(blight.timer_table(), TIMED_FUNCTION_TABLE_CORE);
        blight.core_mode(false);
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
