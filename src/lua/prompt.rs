use mlua::{Function, Table, UserData};

use crate::event::Event;

use super::{
    backend::Backend,
    constants::{BACKEND, PROMPT_CONTENT, PROMPT_CURSOR_INDEX, PROMPT_INPUT_LISTENER_TABLE},
};

#[derive(Debug, Clone)]
pub struct Prompt {}

impl UserData for Prompt {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("set", |ctx, line: String| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend
                .writer
                .send(Event::SetPromptInput(line.clone()))
                .unwrap();
            ctx.set_named_registry_value(PROMPT_CONTENT, line)?;
            Ok(())
        });
        methods.add_function("get", |ctx, ()| -> mlua::Result<String> {
            ctx.named_registry_value(PROMPT_CONTENT)
        });
        methods.add_function("get_cursor_pos", |ctx, ()| -> mlua::Result<usize> {
            let result = ctx.named_registry_value(PROMPT_CURSOR_INDEX);
            if let Ok(pos) = result {
                Ok(pos + 1)
            } else {
                result
            }
        });
        methods.add_function("set_cursor_pos", |ctx, pos: usize| {
            let pos = if pos > 0 { pos - 1 } else { pos };
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::SetPromptCursorPos(pos)).unwrap();
            Ok(())
        });
        methods.add_function(
            "add_prompt_listener",
            |ctx, func: Function| -> mlua::Result<()> {
                let table: Table = ctx.named_registry_value(PROMPT_INPUT_LISTENER_TABLE)?;
                table.set(table.raw_len() + 1, func)?;
                Ok(())
            },
        );
    }
}
