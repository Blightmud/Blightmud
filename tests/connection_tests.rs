use blightmud::RuntimeConfig;
use common::{join_blightmud, Server};

mod common;

#[test]
fn test_connect() {
    let mut server = Server::bind(0);

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
    assert!(handle.join().is_ok(), "Blightmud didn't exit cleanly");
}

#[test]
fn test_connect_world() {
    let mut server = Server::bind(9876);

    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.script = Some("tests/add_world.lua".to_string());
    rt.world = Some("test_world".to_string());
    rt.integration_test = true;
    let handle = common::start_blightmud(rt);

    let connection = server.listen();
    assert!(connection.is_ok());
    let connection = connection.unwrap();
    assert!(connection.stream.is_some());
    connection.close();
    join_blightmud(handle);
}
