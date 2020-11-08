use rlua::{Function, Table, UserData, UserDataMethods};

use super::constants::{MUD_INPUT_LISTENER_TABLE, MUD_OUTPUT_LISTENER_TABLE};

pub struct Mud {}

impl Mud {
    pub fn new() -> Self {
        Self {}
    }
}

impl UserData for Mud {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method(
            "add_output_listener",
            |ctx, _, func: Function| -> rlua::Result<()> {
                let table: Table = ctx.globals().get(MUD_OUTPUT_LISTENER_TABLE)?;
                table.set(table.raw_len() + 2, func)?;
                Ok(())
            },
        );
        methods.add_method(
            "add_input_listener",
            |ctx, _, func: Function| -> rlua::Result<()> {
                let table: Table = ctx.globals().get(MUD_INPUT_LISTENER_TABLE)?;
                table.set(table.raw_len() + 1, func)?;
                Ok(())
            },
        );
    }
}
