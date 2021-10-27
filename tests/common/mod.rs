use std::thread;

use blightmud::RuntimeConfig;

pub mod server;
pub use server::Server;

pub fn start_blightmud(rt: RuntimeConfig) {
    thread::spawn(|| {
        blightmud::start(rt);
    });
}

#[allow(dead_code)]
pub fn random_port() -> u16 {
    portpicker::pick_unused_port().unwrap()
}
