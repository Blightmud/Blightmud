use blightmud::RuntimeConfig;
use common::{join_blightmud, Server};

mod common;

#[test]
fn test_connect() {
    let mut server = Server::bind(0);

    println!("Test server running at: {}", server.local_addr);
    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.connect = Some(format!("{}", server.local_addr));
    rt.integration_test = true;
    let handle = common::start_blightmud(rt);

    let connection = server.listen();
    assert!(connection.is_ok());
    let connection = connection.unwrap();
    assert!(connection.stream.is_some());
    connection.close();
    join_blightmud(handle)
}

#[test]
fn test_connect_world() {
    let mut server = Server::bind(0);

    println!(
        "Test server running at: {} {}",
        server.local_addr.ip(),
        server.local_addr.port()
    );
    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.integration_test = true;
    rt.eval = Some(format!(
        r#"
local ok, server = pcall(servers.get, "test_world")
if ok then
    servers.remove(server.name)
end
servers.add("test_world", "{0}", {1})
server = servers.get("test_world")
mud.connect(server.host, server.port)
mud.on_disconnect(function ()
    servers.remove("test_world")
end)
    "#,
        server.local_addr.ip(),
        server.local_addr.port()
    ));
    let handle = common::start_blightmud(rt);

    let connection = server.listen();
    assert!(connection.is_ok());
    let connection = connection.unwrap();
    assert!(connection.stream.is_some());
    connection.close();
    join_blightmud(handle);
}
