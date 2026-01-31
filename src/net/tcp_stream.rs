use crate::{event::Event, model::Connection, session::Session};
use libmudtelnet::bytes::Bytes;
use log::{debug, error};
use std::{net::TcpStream, sync::mpsc::Receiver, thread};

use super::event_loop::NetworkEventLoop;
use super::tls::create_tls_connection;

pub const BUFFER_SIZE: usize = 32 * 1024;

pub fn spawn_connect_thread(
    mut session: Session,
    connection: Connection,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("connect-thread".to_string())
        .spawn(move || {
            let Connection {
                host,
                port,
                tls,
                verify_cert,
                name,
            } = connection;
            // Set the name on the connection before connecting
            if let Ok(mut conn) = session.connection.lock() {
                conn.name = name;
            }
            if !session.connect(&host, port, tls, verify_cert.into()) {
                session
                    .main_writer
                    .send(Event::Error(format!("Failed to connect to {host}:{port}")))
                    .unwrap();
            }
        })
        .unwrap()
}

/// Spawn a single network thread using mio-based event loop.
/// This replaces the separate receive and transmit threads to fix TLS race conditions.
pub fn spawn_network_thread(
    session: Session,
    stream: TcpStream,
    tls: bool,
    host: &str,
    tls_validation: super::tls::CertificateValidation,
    transmit_receiver: Receiver<Option<Bytes>>,
) -> thread::JoinHandle<()> {
    let host = host.to_string();
    thread::Builder::new()
        .name("network-event-loop".to_string())
        .spawn(move || {
            debug!("Network event loop thread starting (tls: {})", tls);

            let mut event_loop = if tls {
                // Create TLS connection
                let tls_conn = match create_tls_connection(&host, tls_validation) {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("Failed to create TLS connection: {}", e);
                        let _ = session
                            .main_writer
                            .send(Event::Error(format!("TLS initialization failed: {}", e)));
                        let _ = session.main_writer.send(Event::Disconnect);
                        return;
                    }
                };

                match NetworkEventLoop::new_tls(
                    stream,
                    tls_conn,
                    transmit_receiver,
                    session.clone(),
                ) {
                    Ok(el) => el,
                    Err(e) => {
                        error!("Failed to create TLS event loop: {}", e);
                        let _ = session
                            .main_writer
                            .send(Event::Error(format!("Event loop creation failed: {}", e)));
                        let _ = session.main_writer.send(Event::Disconnect);
                        return;
                    }
                }
            } else {
                match NetworkEventLoop::new_plain(stream, transmit_receiver, session.clone()) {
                    Ok(el) => el,
                    Err(e) => {
                        error!("Failed to create event loop: {}", e);
                        let _ = session
                            .main_writer
                            .send(Event::Error(format!("Event loop creation failed: {}", e)));
                        let _ = session.main_writer.send(Event::Disconnect);
                        return;
                    }
                }
            };

            event_loop.run();
            debug!("Network event loop thread exiting");
        })
        .unwrap()
}
