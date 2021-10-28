use std::net::Shutdown;

use blightmud::RuntimeConfig;
use common::Server;

mod common;

#[test]
fn test_connect() {
    let mut server = Server::bind(0);

    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.connect = Some(format!("{}", server.local_addr));
    common::start_blightmud(rt);

    let connection = server.listen();
    assert!(connection.is_ok());
    let connection = connection.unwrap();
    assert!(connection.stream.is_some());
    let _ = connection.stream.unwrap().shutdown(Shutdown::Both);
}
