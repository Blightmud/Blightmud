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
        // Both of these are 1-based, matching `get_cursor_pos` above and Lua
        // convention generally: the first row is row 1.
        //
        // They exist rather than having Lua scan `prompt.get()` itself because
        // `get_cursor_pos` is a *character* index while Lua's `string.sub` is
        // *byte*-based — a scan would split at the wrong point on any
        // non-ASCII input.
        methods.add_function("cursor_row", |ctx, ()| -> mlua::Result<usize> {
            let content: String = ctx.named_registry_value(PROMPT_CONTENT)?;
            let pos: usize = ctx.named_registry_value(PROMPT_CURSOR_INDEX)?;
            Ok(content.chars().take(pos).filter(|c| *c == '\n').count() + 1)
        });
        methods.add_function("row_count", |ctx, ()| -> mlua::Result<usize> {
            let content: String = ctx.named_registry_value(PROMPT_CONTENT)?;
            Ok(content.chars().filter(|c| *c == '\n').count() + 1)
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

#[cfg(test)]
mod test_prompt {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use mlua::{Lua, Table};

    use crate::{
        event::Event,
        lua::{
            backend::Backend,
            constants::{
                BACKEND, PROMPT_CONTENT, PROMPT_CURSOR_INDEX, PROMPT_INPUT_LISTENER_TABLE,
            },
        },
    };

    use super::Prompt;

    fn setup_lua() -> (Lua, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let lua = Lua::new();
        lua.set_named_registry_value(BACKEND, backend).unwrap();
        lua.set_named_registry_value(PROMPT_CONTENT, "".to_string())
            .unwrap();
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 0usize)
            .unwrap();
        let listener_table = lua.create_table().unwrap();
        lua.set_named_registry_value(PROMPT_INPUT_LISTENER_TABLE, listener_table)
            .unwrap();
        lua.globals().set("prompt", Prompt {}).unwrap();
        (lua, reader)
    }

    #[test]
    fn test_prompt_set() {
        let (lua, reader) = setup_lua();
        lua.load("prompt.set('hello world')").exec().unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::SetPromptInput("hello world".to_string()))
        );
    }

    #[test]
    fn test_prompt_get() {
        let (lua, _reader) = setup_lua();
        lua.set_named_registry_value(PROMPT_CONTENT, "test content".to_string())
            .unwrap();
        let value: String = lua.load("return prompt.get()").call(()).unwrap();
        assert_eq!(value, "test content");
    }

    #[test]
    fn test_prompt_get_cursor_pos() {
        let (lua, _reader) = setup_lua();
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 5usize)
            .unwrap();
        let pos: usize = lua.load("return prompt.get_cursor_pos()").call(()).unwrap();
        assert_eq!(pos, 6); // +1 for Lua 1-based indexing
    }

    #[test]
    fn test_prompt_get_cursor_pos_zero() {
        let (lua, _reader) = setup_lua();
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 0usize)
            .unwrap();
        let pos: usize = lua.load("return prompt.get_cursor_pos()").call(()).unwrap();
        assert_eq!(pos, 1); // +1 for Lua 1-based indexing
    }

    #[test]
    fn test_prompt_set_cursor_pos() {
        let (lua, reader) = setup_lua();
        lua.load("prompt.set_cursor_pos(5)").exec().unwrap();
        assert_eq!(reader.recv(), Ok(Event::SetPromptCursorPos(4))); // -1 for 0-based indexing
    }

    #[test]
    fn test_prompt_set_cursor_pos_zero() {
        let (lua, reader) = setup_lua();
        lua.load("prompt.set_cursor_pos(0)").exec().unwrap();
        assert_eq!(reader.recv(), Ok(Event::SetPromptCursorPos(0)));
    }

    #[test]
    fn test_prompt_add_listener() {
        let (lua, _reader) = setup_lua();
        lua.load("prompt.add_prompt_listener(function() end)")
            .exec()
            .unwrap();
        let table: Table = lua
            .named_registry_value(PROMPT_INPUT_LISTENER_TABLE)
            .unwrap();
        assert_eq!(table.raw_len(), 1);
    }

    #[test]
    fn test_prompt_add_multiple_listeners() {
        let (lua, _reader) = setup_lua();
        lua.load("prompt.add_prompt_listener(function() end)")
            .exec()
            .unwrap();
        lua.load("prompt.add_prompt_listener(function() end)")
            .exec()
            .unwrap();
        lua.load("prompt.add_prompt_listener(function() end)")
            .exec()
            .unwrap();
        let table: Table = lua
            .named_registry_value(PROMPT_INPUT_LISTENER_TABLE)
            .unwrap();
        assert_eq!(table.raw_len(), 3);
    }

    #[test]
    fn test_prompt_rows_are_one_based() {
        let (lua, _reader) = setup_lua();
        lua.set_named_registry_value(PROMPT_CONTENT, "one\ntwo\nthree".to_string())
            .unwrap();

        // Cursor on the first row.
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 0usize)
            .unwrap();
        assert_eq!(
            lua.load("return prompt.cursor_row()")
                .call::<usize>(())
                .unwrap(),
            1
        );

        // Just before the first newline is still row 1; just after is row 2.
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 3usize)
            .unwrap();
        assert_eq!(
            lua.load("return prompt.cursor_row()")
                .call::<usize>(())
                .unwrap(),
            1
        );
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 4usize)
            .unwrap();
        assert_eq!(
            lua.load("return prompt.cursor_row()")
                .call::<usize>(())
                .unwrap(),
            2
        );

        assert_eq!(
            lua.load("return prompt.row_count()")
                .call::<usize>(())
                .unwrap(),
            3
        );
    }

    /// The common case: no row breaks means one row, so the up/down bindings
    /// fall straight through to history exactly as they did before.
    #[test]
    fn test_prompt_rows_without_newlines() {
        let (lua, _reader) = setup_lua();
        lua.set_named_registry_value(PROMPT_CONTENT, "plain".to_string())
            .unwrap();
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 3usize)
            .unwrap();
        assert_eq!(
            lua.load("return prompt.cursor_row()")
                .call::<usize>(())
                .unwrap(),
            1
        );
        assert_eq!(
            lua.load("return prompt.row_count()")
                .call::<usize>(())
                .unwrap(),
            1
        );
    }

    /// cursor_row counts characters, not bytes — a byte-based scan in Lua
    /// would split multi-byte input at the wrong point.
    #[test]
    fn test_prompt_cursor_row_counts_characters() {
        let (lua, _reader) = setup_lua();
        lua.set_named_registry_value(PROMPT_CONTENT, "\u{4e2d}\u{6587}\nx".to_string())
            .unwrap();
        // Two CJK characters then the newline: char index 2 is still row 1.
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 2usize)
            .unwrap();
        assert_eq!(
            lua.load("return prompt.cursor_row()")
                .call::<usize>(())
                .unwrap(),
            1
        );
        lua.set_named_registry_value(PROMPT_CURSOR_INDEX, 3usize)
            .unwrap();
        assert_eq!(
            lua.load("return prompt.cursor_row()")
                .call::<usize>(())
                .unwrap(),
            2
        );
    }
}
