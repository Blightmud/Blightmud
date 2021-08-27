use crate::io::SaveData;
use crate::model::{Connection, Servers as MServers};
use mlua::{UserData, UserDataMethods};

use mlua::prelude::ToLua;

#[cfg(test)]
use mockall::automock;

pub struct Server {
    name: String,
    connection: Connection,
}

impl UserData for Server {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_meta_method(
            mlua::MetaMethod::Index,
            |ctx, this, key: String| -> mlua::Result<mlua::Value> {
                match key.as_str() {
                    "name" => Ok(this.name.clone().to_lua(ctx)?),
                    "host" => Ok(this.connection.host.clone().to_lua(ctx)?),
                    "port" => Ok(this.connection.port.to_lua(ctx)?),
                    "tls" => Ok(this.connection.tls.to_lua(ctx)?),
                    "verify_cert" => Ok(this.connection.verify_cert.to_lua(ctx)?),
                    _ => Err(mlua::Error::external(format!("Invalid index: {}", key))),
                }
            },
        );
    }
}

pub struct Servers {}

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
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function(
            "add",
            |_,
             (name, host, port, tls, verify): (String, String, u16, bool, Option<bool>)|
             -> mlua::Result<()> {
                let mut servers = ServerLoader::get()?;

                #[allow(clippy::map_entry)]
                if servers.contains_key(&name) {
                    Err(mlua::Error::external(format!(
                        "Saved server already exists for {}",
                        name
                    )))
                } else {
                    let connection = Connection {
                        host,
                        port,
                        tls,
                        verify_cert: verify.unwrap_or(false),
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
                    "Saved server does not exist: {}",
                    name
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
                        "Failed to read server: {}",
                        name
                    )))
                }
            } else {
                Err(mlua::Error::external(format!(
                    "Saved server does not exist: {}",
                    name
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
mod test_servers {

    use std::collections::HashMap;

    use mlua::Lua;

    use super::*;

    fn get_lua() -> Lua {
        let state = Lua::new();
        state.globals().set("servers", Servers {}).unwrap();
        state
    }

    #[test]
    #[ignore]
    fn test_save() {
        let ctx = MockServerLoader::get_context();
        ctx.expect().returning(|| {
            let mut servers = HashMap::new();
            servers.insert(
                "test".to_string(),
                Connection {
                    host: "test.com".to_string(),
                    port: 4000,
                    tls: false,
                    verify_cert: false,
                },
            );
            Ok(servers)
        });
        let lua = get_lua();
        let lua_code = r#"
        if pcall(function () servers.add("test", "test.com", 4000, false) end) then
            return true
        else
            return false
        end
        "#;

        assert_eq!(lua.load(lua_code).call::<_, bool>(()).unwrap(), true);
        assert_eq!(lua.load(lua_code).call::<_, bool>(()).unwrap(), false);
    }
}
