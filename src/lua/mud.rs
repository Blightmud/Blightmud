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
                table.set(table.raw_len() + 1, func)?;
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

#[cfg(test)]
mod test_mud {
    use super::Mud;

    #[test]
    fn test_output_register() {
        test_lua!("mud" => Mud::new());
        run_lua!("__output_listeners = {}");
        assert_lua!(u32, "#__output_listeners", 0);
        run_lua!("mud:add_output_listener(function () end)");
        assert_lua!(u32, "#__output_listeners", 1);
    }

    #[test]
    fn test_input_register() {
        test_lua!("mud" => Mud::new());
        run_lua!("__input_listeners = {}");
        assert_lua!(u32, "#__input_listeners", 0);
        run_lua!("mud:add_input_listener(function () end)");
        assert_lua!(u32, "#__input_listeners", 1);
    }
}
