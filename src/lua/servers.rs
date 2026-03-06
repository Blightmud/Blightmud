use crate::io::SaveData;
use crate::model::{Connection, Servers as MServers};
use mlua::{IntoLua, UserData, UserDataMethods};

#[cfg(test)]
use mockall::automock;

pub struct Server {
    name: String,
    connection: Connection,
}

impl UserData for Server {
    fn add_methods<T: UserDataMethods<Self>>(methods: &mut T) {
        methods.add_meta_method(
            mlua::MetaMethod::Index,
            |ctx, this, key: String| -> mlua::Result<mlua::Value> {
                match key.as_str() {
                    "name" => Ok(this.name.clone().into_lua(ctx)?),
                    "host" => Ok(this.connection.host.clone().into_lua(ctx)?),
                    "port" => Ok(this.connection.port.into_lua(ctx)?),
                    "tls" => Ok(this.connection.tls.into_lua(ctx)?),
                    "verify_cert" => Ok(this.connection.verify_cert.into_lua(ctx)?),
                    _ => Err(mlua::Error::external(format!("Invalid index: {key}"))),
                }
            },
        );
    }
}

pub struct Servers();

struct ServerLoader {}

#[cfg_attr(test, automock)]
impl ServerLoader {
    fn get() -> mlua::Result<MServers> {
        if let Ok(servers) = MServers::try_load() {
            Ok(servers)
        } else {
            Err(mlua::Error::external(
                "Unable to read servers.ron from disk".to_string(),
            ))
        }
    }
}

impl UserData for Servers {
    fn add_methods<T: UserDataMethods<Self>>(methods: &mut T) {
        methods.add_function(
            "add",
            |_,
             (name, host, port, tls, verify): (String, String, u16, bool, Option<bool>)|
             -> mlua::Result<()> {
                let mut servers = ServerLoader::get()?;

                #[allow(clippy::map_entry)]
                if servers.contains_key(&name) {
                    Err(mlua::Error::external(format!(
                        "Saved server already exists for {name}"
                    )))
                } else {
                    let connection = Connection {
                        host,
                        port,
                        tls,
                        verify_cert: verify.unwrap_or(false),
                        name: None,
                    };
                    servers.insert(name, connection);
                    servers.save();
                    Ok(())
                }
            },
        );
        methods.add_function("remove", |_, name: String| -> mlua::Result<()> {
            let mut servers = ServerLoader::get()?;
            if servers.remove(&name).is_some() {
                servers.save();
                Ok(())
            } else {
                Err(mlua::Error::external(format!(
                    "Saved server does not exist: {name}"
                )))
            }
        });
        methods.add_function("get", |_, name: String| -> mlua::Result<Server> {
            let servers = ServerLoader::get()?;
            if servers.contains_key(&name) {
                if let Some(connection) = servers.get(&name) {
                    Ok(Server {
                        name,
                        connection: connection.clone(),
                    })
                } else {
                    Err(mlua::Error::external(format!(
                        "Failed to read server: {name}"
                    )))
                }
            } else {
                Err(mlua::Error::external(format!(
                    "Saved server does not exist: {name}"
                )))
            }
        });
        methods.add_function("get_all", |_, ()| -> mlua::Result<Vec<Server>> {
            let servers = ServerLoader::get()?;
            Ok(servers
                .iter()
                .map(|(name, conn)| Server {
                    name: name.to_string(),
                    connection: conn.clone(),
                })
                .collect())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn get_lua() -> Lua {
        let lua = Lua::new();
        lua.globals().set("servers", Servers()).unwrap();
        lua
    }

    #[test]
    fn test_server_index_invalid() {
        let server = Server {
            name: "test_server".to_string(),
            connection: Connection::new("example.com", 4000, false, false),
        };
        let lua = Lua::new();
        lua.globals().set("server", server).unwrap();

        let result: Result<mlua::Value, _> = lua.load("return server.invalid_key").call(());
        assert!(result.is_err());
    }

    #[test]
    fn test_server_index_name() {
        let server = Server {
            name: "test_server".to_string(),
            connection: Connection::new("example.com", 4000, false, false),
        };
        let lua = Lua::new();
        lua.globals().set("server", server).unwrap();

        let name: String = lua.load("return server.name").call(()).unwrap();
        assert_eq!(name, "test_server");
    }

    #[test]
    fn test_server_index_host() {
        let server = Server {
            name: "test_server".to_string(),
            connection: Connection::new("example.com", 4000, false, false),
        };
        let lua = Lua::new();
        lua.globals().set("server", server).unwrap();

        let host: String = lua.load("return server.host").call(()).unwrap();
        assert_eq!(host, "example.com");
    }

    #[test]
    fn test_server_index_port() {
        let server = Server {
            name: "test_server".to_string(),
            connection: Connection::new("example.com", 4000, false, false),
        };
        let lua = Lua::new();
        lua.globals().set("server", server).unwrap();

        let port: u16 = lua.load("return server.port").call(()).unwrap();
        assert_eq!(port, 4000);
    }

    #[test]
    fn test_server_index_tls() {
        let server = Server {
            name: "test_server".to_string(),
            connection: Connection::new("example.com", 4000, true, false),
        };
        let lua = Lua::new();
        lua.globals().set("server", server).unwrap();

        let tls: bool = lua.load("return server.tls").call(()).unwrap();
        assert!(tls);
    }

    #[test]
    fn test_server_index_verify_cert() {
        let server = Server {
            name: "test_server".to_string(),
            connection: Connection::new("example.com", 4000, true, true),
        };
        let lua = Lua::new();
        lua.globals().set("server", server).unwrap();

        let verify: bool = lua.load("return server.verify_cert").call(()).unwrap();
        assert!(verify);
    }

    #[test]
    fn test_servers_get_all() {
        let lua = get_lua();
        // get_all should return a vec (possibly empty if no servers configured)
        let result: Result<Vec<mlua::Value>, _> = lua.load("return servers.get_all()").call(());
        assert!(result.is_ok());
    }
}
