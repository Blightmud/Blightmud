use blightmud::RuntimeConfig;
use common::Server;
mod common;

#[test]
fn validate_assertion_fail() {
    let mut server = Server::bind(0);

    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.connect = Some(format!("{}", server.local_addr));
    rt.integration_test = true;
    rt.script = Some("tests/assertion_fail.lua".to_string());
    let handle = common::start_blightmud(rt);

    let connection = server.listen();
    assert!(connection.is_ok());
    let connection = connection.unwrap();
    assert!(connection.stream.is_some());
    connection.close();
    assert!(handle.join().unwrap().is_err(), "Blightmud exited cleanly");
}
