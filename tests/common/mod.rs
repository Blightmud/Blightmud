use std::thread::{self, JoinHandle};

use anyhow::Result;
use blightmud::RuntimeConfig;

pub mod server;
pub use server::Server;

use self::server::Connection;

pub fn start_blightmud(rt: RuntimeConfig) -> JoinHandle<Result<()>> {
    thread::spawn(|| -> Result<()> { blightmud::start(rt) })
}

#[allow(dead_code)]
pub fn join_blightmud(handle: JoinHandle<Result<()>>) {
    assert!(
        handle.join().unwrap().is_ok(),
        "Blightmud didn't exit cleanly"
    );
}

#[allow(dead_code)]
pub fn setup(script_file: Option<String>) -> (Connection, JoinHandle<Result<()>>) {
    let mut server = Server::bind(0);

    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.script = script_file;
    rt.eval = Some(include_str!("quit_on_disconnect.lua").to_string());
    rt.integration_test = true;
    println!("Test server running at: {}", server.local_addr);
    rt.connect = Some(format!("{}", server.local_addr));
    let handle = start_blightmud(rt);

    let connection = server.listen();
    assert!(connection.is_ok());
    let connection = connection.unwrap();
    assert!(connection.stream.is_some());

    (connection, handle)
}
