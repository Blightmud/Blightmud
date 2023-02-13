use std::{
    io::Write,
    net::{Shutdown, TcpStream},
};

use mlua::{UserData, UserDataMethods};

use crate::{
    event::Event,
    lua::{backend::Backend, constants::BACKEND},
    net::open_tcp_stream,
};

pub struct SocketLib;

impl UserData for SocketLib {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function(
            "connect",
            |ctx, (host, port): (String, u16)| -> mlua::Result<Option<Socket>> {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                if let Ok(connection) = open_tcp_stream(&host, port) {
                    Ok(Some(Socket { connection }))
                } else {
                    backend
                        .writer
                        .send(Event::Error(format!("Unable to connect to {host}:{port}")))
                        .unwrap();
                    Ok(None)
                }
            },
        );
    }
}

pub struct Socket {
    connection: TcpStream,
}

impl UserData for Socket {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method_mut("send", |_, this, data: String| {
            let _ = this.connection.write(data.as_bytes());
            Ok(())
        });
        methods.add_method_mut("close", |_, this, ()| {
            let _ = this.connection.shutdown(Shutdown::Both);
            Ok(())
        });
    }
}
