use std::{
    io::Write,
    net::{Shutdown, TcpStream},
};

use rlua::{UserData, UserDataMethods};

pub struct SocketLib;

impl UserData for SocketLib {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function(
            "connect",
            |_, (host, port): (String, u32)| -> rlua::Result<Option<Socket>> {
                if let Ok(connection) = TcpStream::connect(format!("{}:{}", host, port)) {
                    Ok(Some(Socket { connection }))
                } else {
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
