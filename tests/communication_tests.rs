use std::{env, thread::JoinHandle};

use blightmud::{RuntimeConfig, PROJECT_NAME, VERSION};
use common::{server::Connection, Server};
use libtelnet_rs::telnet::{op_command::*, op_option::*};

mod common;

fn setup() -> (Connection, JoinHandle<()>) {
    let mut server = Server::bind(0);

    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.no_panic_hook = true;
    println!("Test server running at: {}", server.local_addr);
    rt.connect = Some(format!("{}", server.local_addr));
    let handle = common::start_blightmud(rt);

    let connection = server.listen();
    assert!(connection.is_ok());
    let connection = connection.unwrap();
    assert!(connection.stream.is_some());

    (connection, handle)
}

#[test]
fn test_ttype_negotiation() -> std::io::Result<()> {
    let (mut connection, handle) = setup();

    connection.send(&[IAC, WILL, TTYPE]);
    assert_eq!(connection.recv(), &[IAC, DO, TTYPE]);
    connection.send(&[IAC, SB, TTYPE, SEND, IAC, SE]);
    assert_eq!(
        connection.recv(),
        vec![&[IAC, SB, TTYPE, IS][..], b"BLIGHTMUD", &[IAC, SE][..]].concat()
    );
    connection.send(&[IAC, SB, TTYPE, SEND, IAC, SE]);
    let term = if let Ok(term) = env::var("TERM") {
        term.to_ascii_uppercase()
    } else {
        "XTERM-256COLOR".to_string()
    };
    assert_eq!(
        connection.recv(),
        vec![
            &[IAC, SB, TTYPE, IS][..],
            &term.bytes().collect::<Vec<u8>>()[..],
            &[IAC, SE][..]
        ]
        .concat(),
    );

    connection.send(&[IAC, SB, TTYPE, SEND, IAC, SE]);
    assert_eq!(
        connection.recv(),
        vec![&[IAC, SB, TTYPE, IS][..], b"MTTS 271", &[IAC, SE][..]].concat()
    );

    connection.close();
    handle.join().unwrap();
    Ok(())
}

#[test]
fn test_gmcp_negotiation() -> std::io::Result<()> {
    let (mut connection, handle) = setup();

    connection.send(&[IAC, WILL, GMCP]);
    assert_eq!(connection.recv(), &[IAC, DO, GMCP]);
    let response = connection.recv();
    let expected1 = format!(
        "Core.Hello {{\"Version\":\"{}\",\"Client\":\"{}\"}}",
        VERSION, PROJECT_NAME
    );
    let expected2 = format!(
        "Core.Hello {{\"Client\":\"{}\",\"Version\":\"{}\"}}",
        PROJECT_NAME, VERSION
    );
    let success = response
        == vec![
            &[IAC, SB, GMCP][..],
            &expected1.bytes().collect::<Vec<u8>>()[..],
            &[IAC, SE][..],
        ]
        .concat()
        || response
            == vec![
                &[IAC, SB, GMCP][..],
                &expected2.bytes().collect::<Vec<u8>>()[..],
                &[IAC, SE][..],
            ]
            .concat();

    assert!(success);

    connection.close();
    handle.join().unwrap();
    Ok(())
}
