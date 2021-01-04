use std::sync::mpsc::Sender;

use log::debug;
use rlua::{AnyUserData, Table, UserData, UserDataMethods};

use crate::{event::Event, io::exec};

use super::{
    constants::{PROTO_ENABLED_LISTENERS_TABLE, PROTO_SUBNEG_LISTENERS_TABLE},
    exec_response::ExecResponse,
};

#[derive(Debug, Clone)]
pub struct Core {
    main_writer: Sender<Event>,
    next_id: u32,
}

impl Core {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            next_id: 0,
        }
    }

    fn next_index(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }
}

impl UserData for Core {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("enable_protocol", |ctx, proto: u8| {
            let this_aux = ctx.globals().get::<_, AnyUserData>("core")?;
            let this = this_aux.borrow_mut::<Core>()?;
            this.main_writer.send(Event::EnableProto(proto)).unwrap();
            Ok(())
        });
        methods.add_function("disable_protocol", |ctx, proto: u8| {
            let this_aux = ctx.globals().get::<_, AnyUserData>("core")?;
            let this = this_aux.borrow_mut::<Core>()?;
            this.main_writer.send(Event::DisableProto(proto)).unwrap();
            Ok(())
        });
        methods.add_function_mut("on_protocol_enabled", |ctx, cb: rlua::Function| {
            let globals = ctx.globals();
            let table: Table = globals.get(PROTO_ENABLED_LISTENERS_TABLE)?;
            let this_aux = ctx.globals().get::<_, AnyUserData>("core")?;
            let mut this = this_aux.borrow_mut::<Core>()?;
            table.set(this.next_index(), cb)?;
            globals.set(PROTO_ENABLED_LISTENERS_TABLE, table)?;
            Ok(())
        });
        methods.add_function_mut("subneg_recv", |ctx, cb: rlua::Function| {
            let globals = ctx.globals();
            let table: Table = globals.get(PROTO_SUBNEG_LISTENERS_TABLE)?;
            let this_aux = ctx.globals().get::<_, AnyUserData>("core")?;
            let mut this = this_aux.borrow_mut::<Core>()?;
            table.set(this.next_index(), cb)?;
            globals.set(PROTO_SUBNEG_LISTENERS_TABLE, table)?;
            Ok(())
        });
        methods.add_function_mut("subneg_send", |ctx, (proto, bytes): (u8, Table)| {
            let this_aux = ctx.globals().get::<_, AnyUserData>("core")?;
            let this = this_aux.borrow_mut::<Core>()?;
            let data = bytes
                .pairs::<i32, u8>()
                .filter_map(Result::ok)
                .map(|pair| pair.1)
                .collect::<Vec<u8>>();
            debug!("lua subneg: {}", String::from_utf8_lossy(&data).to_mut());
            this.main_writer
                .send(Event::ProtoSubnegSend(proto, data))
                .unwrap();
            Ok(())
        });
        methods.add_function(
            "exec",
            |_, cmd: String| -> Result<ExecResponse, rlua::Error> {
                match exec(&cmd) {
                    Ok(output) => Ok(ExecResponse::from(output)),
                    Err(err) => Err(rlua::Error::RuntimeError(err.to_string())),
                }
            },
        );
    }
}
