use mlua::{Result as LuaResult, String as LuaString, Table, UserData};
use std::ops::Not;

use super::{
    backend::Backend,
    constants::{BACKEND, PROMPT_CONTENT, PROMPT_MASK_CONTENT},
};
use crate::event::Event;
use crate::model;

#[derive(Debug, Clone)]
pub struct PromptMask {}

impl UserData for PromptMask {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function(
            "set",
            |ctx, (data, mask): (LuaString, Table)| -> LuaResult<bool> {
                let prompt_data: String = ctx.named_registry_value(PROMPT_CONTENT).unwrap();
                let mask_data = data.to_str().unwrap();
                if prompt_data != mask_data {
                    return Ok(false);
                }
                let prompt_mask = model::PromptMask::from(mask);
                let valid = prompt_mask
                    .iter()
                    .map(|(offset, _)| *offset as usize)
                    .any(|offset| {
                        offset > prompt_data.len() + 1 || !prompt_data.is_char_boundary(offset)
                    })
                    .not();
                if valid {
                    ctx.named_registry_value::<_, Backend>(BACKEND)?
                        .writer
                        .send(Event::SetPromptMask(prompt_mask))
                        .unwrap();
                }
                Ok(valid)
            },
        );
        methods.add_function("clear", |ctx, ()| -> LuaResult<()> {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::ClearPromptMask).unwrap();
            Ok(())
        });
        methods.add_function("get", |ctx, ()| -> LuaResult<Table> {
            ctx.named_registry_value(PROMPT_MASK_CONTENT)
        });
    }
}

#[cfg(test)]
mod test_prompt_mask {
    use crate::event::Event;
    use crate::lua::backend::Backend;
    use crate::lua::constants::{BACKEND, PROMPT_CONTENT, PROMPT_MASK_CONTENT};
    use crate::lua::prompt_mask::PromptMask;
    use crate::model;
    use mlua::{Lua, Table};
    use std::collections::BTreeMap;
    use std::sync::mpsc::{channel, Receiver, Sender};

    fn get_lua_state(prompt_content: &str) -> (Lua, Receiver<Event>) {
        let lua = Lua::new();
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend { writer };
        let prompt_mask = PromptMask {};
        lua.globals().set("prompt_mask", prompt_mask).unwrap();
        lua.set_named_registry_value(PROMPT_CONTENT, prompt_content)
            .unwrap();
        lua.set_named_registry_value(BACKEND, backend).unwrap();
        (lua, reader)
    }

    #[test]
    fn test_set_mask_valid() {
        let prompt_state = "hi hello bye";
        let (lua, reader) = get_lua_state(prompt_state);
        // Note: we expect the loaded mask's indexes to have been converted to 0-indexing.
        let expected_mask =
            model::PromptMask::from(BTreeMap::from([(3, "*".to_string()), (5, "*".to_string())]));
        let test_script = format!(
            r#"
    local good_mask = {{ 
      [4] = "*", [6] = "*"
    }}
    mask_set = prompt_mask.set({:?}, good_mask)
"#,
            prompt_state
        );
        lua.load(test_script.as_str()).exec().unwrap();
        let mask_set: bool = lua.globals().get("mask_set").unwrap();
        assert_eq!(mask_set, true);
        assert_eq!(reader.recv(), Ok(Event::SetPromptMask(expected_mask)));
    }

    #[test]
    fn test_set_mask_data_stale() {
        let prompt_state = "initial prompt input";
        let (lua, reader) = get_lua_state(prompt_state);
        let test_script = r#"
    local good_mask = {{ [7] = "!" }}
    -- NOTE: data arg doesn't match prompt_state.
    mask_set = prompt_mask.set("diff. prompt input", good_mask)
"#;
        lua.load(test_script).exec().unwrap();
        let mask_set: bool = lua.globals().get("mask_set").unwrap();
        assert_eq!(mask_set, false);
        assert!(reader.try_recv().is_err());
    }

    #[test]
    fn test_set_mask_index_oob() {
        let prompt_state = "hello";
        let (lua, reader) = get_lua_state(prompt_state);
        let test_script = format!(
            r#"
    -- NOTE: Index is out of bounds for current prompt state.
    local bad_mask = {{ [999] = "!" }}
    mask_set = prompt_mask.set({:?}, bad_mask)
"#,
            prompt_state
        );
        lua.load(test_script.as_str()).exec().unwrap();
        let mask_set: bool = lua.globals().get("mask_set").unwrap();
        assert_eq!(mask_set, false);
        assert!(reader.try_recv().is_err());
    }

    #[test]
    fn test_set_mask_index_not_char_boundary() {
        let prompt_state = "hi 😇 bye";
        let (lua, reader) = get_lua_state(prompt_state);
        let test_script = format!(
            r#"
    -- NOTE: Index 5 falls inside of the multi-byte emoji and should fail a is_char_boundary test.
    local bad_mask = {{ [5] = "!" }}
    mask_set = prompt_mask.set({:?}, bad_mask)
"#,
            prompt_state
        );
        lua.load(test_script.as_str()).exec().unwrap();
        let mask_set: bool = lua.globals().get("mask_set").unwrap();
        assert_eq!(mask_set, false);
        assert!(reader.try_recv().is_err());
    }

    #[test]
    fn test_clear_mask() {
        let (lua, reader) = get_lua_state("");
        lua.load("prompt_mask.clear()").exec().unwrap();
        assert_eq!(reader.recv(), Ok(Event::ClearPromptMask));
    }

    #[test]
    fn test_get_mask() {
        let (lua, _reader) = get_lua_state("just some test content");
        let mask = model::PromptMask::from(BTreeMap::from([
            (10, "hi".to_string()),
            (20, "bye".to_string()),
        ]));

        lua.set_named_registry_value(PROMPT_MASK_CONTENT, mask.to_table(&lua).unwrap())
            .unwrap();
        lua.load("mask = prompt_mask.get()").exec().unwrap();
        let result = lua.globals().get::<_, Table>("mask").unwrap();

        assert_eq!(result.get::<i32, String>(11).unwrap(), "hi");
        assert_eq!(result.get::<i32, String>(21).unwrap(), "bye");
    }
}
