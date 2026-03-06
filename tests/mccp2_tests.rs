use common::{join_blightmud, setup};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use libmudtelnet::telnet::{op_command::*, op_option::*};
use std::io::Write;
use std::thread;
use std::time::Duration;

mod common;

/// Compress data using zlib (for MCCP2)
fn compress_zlib(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn test_mccp2_negotiation() {
    let (mut connection, handle) = setup(None);

    // Server sends WILL MCCP2
    connection.send(&[IAC, WILL, MCCP2]);

    // Client should respond with DO MCCP2
    assert_eq!(connection.read(3), &[IAC, DO, MCCP2]);

    connection.close();
    join_blightmud(handle);
}

#[test]
fn test_mccp2_decompression() {
    let (mut connection, handle) = setup(Some("tests/common/mccp2_test.lua".to_string()));

    // Step 1: Negotiate MCCP2
    connection.send(&[IAC, WILL, MCCP2]);
    assert_eq!(connection.read(3), &[IAC, DO, MCCP2]);

    // Step 2: Send subnegotiation to start compression
    // IAC SB MCCP2 IAC SE - after this, all data is compressed
    connection.send(&[IAC, SB, MCCP2, IAC, SE]);

    // Step 3: Send compressed test message
    let test_message = b"MCCP2_TEST_MESSAGE\r\n";
    let compressed = compress_zlib(test_message);
    connection.send(&compressed);

    // Step 4: Wait for the client to process and respond
    // The Lua script will send "MCCP2_OK" when it receives the test message
    thread::sleep(Duration::from_millis(500));
    let response = connection.recv_string();
    assert!(
        response.contains("MCCP2_OK"),
        "Expected MCCP2_OK response, got: {:?}",
        response
    );

    connection.close();
    join_blightmud(handle);
}

#[test]
fn test_mccp2_incremental() {
    let (mut connection, handle) = setup(Some("tests/common/mccp2_test.lua".to_string()));

    // Negotiate MCCP2
    connection.send(&[IAC, WILL, MCCP2]);
    assert_eq!(connection.read(3), &[IAC, DO, MCCP2]);

    // Start compression
    connection.send(&[IAC, SB, MCCP2, IAC, SE]);

    // Create a larger message to ensure it compresses to multiple chunks
    let test_message = b"MCCP2_INCREMENTAL_TEST\r\n";
    let compressed = compress_zlib(test_message);

    // Send compressed data in small chunks to test streaming decompression
    for chunk in compressed.chunks(5) {
        connection.send(chunk);
        thread::sleep(Duration::from_millis(10));
    }

    // Wait for the client to process and respond
    thread::sleep(Duration::from_millis(500));
    let response = connection.recv_string();
    assert!(
        response.contains("MCCP2_INCREMENTAL_OK"),
        "Expected MCCP2_INCREMENTAL_OK response, got: {:?}",
        response
    );

    connection.close();
    join_blightmud(handle);
}
