use common::setup;
use libtelnet_rs::telnet::op_command::*;

mod common;

#[test]
fn timer_test() {
    let (mut connection, handle) = setup();

    assert_eq!(connection.read(2), &[IAC, NOP]);
    assert_eq!(connection.read(2), &[IAC, NOP]);
    assert_eq!(connection.read(2), &[IAC, NOP]);
    assert_eq!(connection.read(2), &[IAC, NOP]);
    assert_eq!(connection.read(2), &[IAC, NOP]);

    connection.close();

    handle.join().unwrap();
}
