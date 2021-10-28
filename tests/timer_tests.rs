use common::setup;
use libtelnet_rs::telnet::op_command::*;

mod common;

#[test]
fn timer_test() {
    let (mut connection, handle) = setup();

    assert_eq!(connection.recv(), &[IAC, NOP]);
    assert_eq!(connection.recv(), &[IAC, NOP]);
    assert_eq!(connection.recv(), &[IAC, NOP]);
    assert_eq!(connection.recv(), &[IAC, NOP]);
    assert_eq!(connection.recv(), &[IAC, NOP]);

    connection.close();

    handle.join().unwrap();
}
