use blightmud::RuntimeConfig;
use common::{join_blightmud, Server};

mod common;

#[test]
#[ignore]
fn test_connect() {
    let mut server = Server::bind(0);

    println!("Test server running at: {}", server.local_addr);
    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.eval = Some(include_str!("common/quit_on_disconnect.lua").to_string());
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
#[ignore]
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
        include_str!("connect_world.lua"),
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

#[test]
#[ignore]
fn test_reconnect_world() {
    let server = Server::bind(0);

    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.integration_test = true;
    rt.connect = Some(server.local_addr.to_string());
    rt.script = Some("tests/test_reconnect.lua".to_string());
    join_blightmud(common::start_blightmud(rt))
}

#[test]
#[ignore]
fn test_is_connected() {
    let server = Server::bind(0);

    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.integration_test = true;
    rt.connect = Some(server.local_addr.to_string());
    rt.script = Some("tests/is_connected.lua".to_string());
    join_blightmud(common::start_blightmud(rt))
}
