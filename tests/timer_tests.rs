use common::setup;
use libmudtelnet::telnet::op_command::*;

mod common;

#[test]
fn timer_test() {
    let (mut connection, handle) = setup(Some("tests/timer_test.lua".to_string()));

    assert_eq!(connection.read(2), &[IAC, NOP]);
    assert_eq!(connection.read(2), &[IAC, NOP]);
    assert_eq!(connection.read(2), &[IAC, NOP]);
    assert_eq!(connection.read(2), &[IAC, NOP]);
    assert_eq!(connection.read(2), &[IAC, NOP]);

    connection.close();

    assert!(handle.join().is_ok(), "Blightmud didn't exit cleanly");
}
