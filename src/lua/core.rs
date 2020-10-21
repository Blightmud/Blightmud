use std::{collections::HashMap, sync::mpsc::Sender};

use log::debug;
use rlua::{Table, UserData, UserDataMethods};

use crate::{event::Event, io::exec};

use super::{
    constants::{PROTO_ENABLED_LISTENERS_TABLE, PROTO_SUBNEG_LISTENERS_TABLE},
    exec_response::ExecResponse,
};

#[derive(Debug, Clone)]
pub struct Core {
    main_writer: Sender<Event>,
    local_storage: HashMap<String, String>,
    next_id: u32,
}

impl Core {
    pub fn new(writer: Sender<Event>) -> Self {
        Self {
            main_writer: writer,
            local_storage: HashMap::new(),
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
        methods.add_method("enable_protocol", |_, this, proto: u8| {
            this.main_writer.send(Event::EnableProto(proto)).unwrap();
            Ok(())
        });
        methods.add_method("disable_protocol", |_, this, proto: u8| {
            this.main_writer.send(Event::DisableProto(proto)).unwrap();
            Ok(())
        });
        methods.add_method_mut("on_protocol_enabled", |ctx, this, cb: rlua::Function| {
            let globals = ctx.globals();
            let table: Table = globals.get(PROTO_ENABLED_LISTENERS_TABLE)?;
            table.set(this.next_index(), cb)?;
            globals.set(PROTO_ENABLED_LISTENERS_TABLE, table)?;
            Ok(())
        });
        methods.add_method_mut("subneg_recv", |ctx, this, cb: rlua::Function| {
            let globals = ctx.globals();
            let table: Table = globals.get(PROTO_SUBNEG_LISTENERS_TABLE)?;
            table.set(this.next_index(), cb)?;
            globals.set(PROTO_SUBNEG_LISTENERS_TABLE, table)?;
            Ok(())
        });
        methods.add_method_mut("subneg_send", |_, this, (proto, bytes): (u8, Table)| {
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
        methods.add_method_mut("store", |_, this, (key, value): (String, String)| {
            debug!("Storing: {}:{}", key, value);
            this.local_storage.insert(key, value);
            Ok(())
        });
        methods.add_method(
            "read",
            |_, this, key: String| -> Result<Option<String>, rlua::Error> {
                debug!("Reading: {}", key);
                Ok(if let Some(val) = this.local_storage.get(&key) {
                    debug!("Read: {}", val);
                    Some(val.to_string())
                } else {
                    debug!("Read: nothing");
                    None
                })
            },
        );
        methods.add_method(
            "exec",
            |_, _, cmd: String| -> Result<ExecResponse, rlua::Error> {
                match exec(&cmd) {
                    Ok(output) => Ok(ExecResponse::from(output)),
                    Err(err) => Err(rlua::Error::RuntimeError(err.to_string())),
                }
            },
        );
    }
}
