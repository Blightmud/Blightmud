use super::{constants::*, ui_event::UiEvent};
use crate::event::Event;
use crate::{model::Line, PROJECT_NAME, VERSION};
use log::debug;
use rlua::{Result as LuaResult, UserData, UserDataMethods, Variadic};
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Blight {
    main_writer: Sender<Event>,
    output_lines: Vec<Line>,
    ui_events: Vec<UiEvent>,
    pub screen_dimensions: (u16, u16),
    pub core_mode: bool,
}

impl Blight {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            output_lines: vec![],
            ui_events: vec![],
            screen_dimensions: (0, 0),
            core_mode: false,
        }
    }

    pub fn core_mode(&mut self, mode: bool) {
        self.core_mode = mode;
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
        methods.add_method("is_core_mode", |_, this, ()| Ok(this.core_mode));
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
