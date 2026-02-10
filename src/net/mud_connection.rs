use anyhow::Result;
use lazy_static::lazy_static;
use log::debug;
use std::{
    net::Shutdown,
    net::TcpStream,
    sync::{atomic::AtomicU16, atomic::Ordering},
};

use crate::net::open_tcp_stream;
use crate::net::tls::CertificateValidation;

/// MudConnection manages the TCP connection to a MUD server.
///
/// With the new mio-based event loop architecture, this struct primarily
/// stores connection metadata and provides connection/disconnection logic.
/// The actual I/O is handled by the NetworkEventLoop.
pub struct MudConnection {
    pub id: u16,
    /// The raw TCP stream, stored here until taken by the event loop
    stream: Option<TcpStream>,
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub tls_validation: CertificateValidation,
    pub name: Option<String>,
    /// Flag indicating whether the stream has been taken by the event loop
    stream_taken: bool,
}

lazy_static! {
    static ref CONNECTION_ID: AtomicU16 = AtomicU16::new(0);
}

fn connection_id() -> u16 {
    CONNECTION_ID.fetch_add(1, Ordering::Relaxed)
}

impl MudConnection {
    pub fn new() -> Self {
        Self {
            id: connection_id(),
            stream: None,
            host: "0.0.0.0".to_string(),
            port: 4000,
            tls: false,
            tls_validation: CertificateValidation::DangerousDisabled,
            name: None,
            stream_taken: false,
        }
    }

    /// Connect to the MUD server.
    ///
    /// This establishes the TCP connection but does not start the event loop.
    /// Call `take_stream()` to get the stream for use with the event loop.
    pub fn connect(
        &mut self,
        host: &str,
        port: u16,
        tls: bool,
        tls_validation: CertificateValidation,
    ) -> Result<()> {
        self.host = host.to_string();
        self.port = port;
        self.tls = tls;
        self.tls_validation = tls_validation;
        self.stream_taken = false;

        debug!(
            "Connecting to {}:{} tls: {} verify: {}",
            host, port, tls, tls_validation
        );

        let stream = open_tcp_stream(&self.host, self.port)?;
        self.stream = Some(stream);
        self.id = connection_id();
        Ok(())
    }

    /// Take the TCP stream for use with the event loop.
    ///
    /// This can only be called once after `connect()`. The stream is moved
    /// to the event loop which handles all I/O.
    pub fn take_stream(&mut self) -> Option<TcpStream> {
        if self.stream_taken {
            return None;
        }
        self.stream_taken = true;
        self.stream.take()
    }

    /// Disconnect from the MUD server.
    ///
    /// If the stream has been taken by the event loop, this just clears
    /// the connection state. The event loop handles the actual disconnection.
    pub fn disconnect(&mut self) -> Result<()> {
        debug!("Disconnecting from {}:{}", self.host, self.port);

        // If we still have the stream (not taken by event loop), shut it down
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(Shutdown::Both);
        }

        self.stream_taken = false;
        debug!("Disconnected from {}:{}", self.host, self.port);
        Ok(())
    }

    /// Check if connected.
    ///
    /// Returns true if we have an active connection (either stream is present
    /// or has been taken by the event loop).
    pub fn connected(&self) -> bool {
        self.stream.is_some() || self.stream_taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mud_connection_new() {
        let conn = MudConnection::new();
        assert_eq!(conn.host, "0.0.0.0");
        assert_eq!(conn.port, 4000);
        assert!(!conn.tls);
        assert!(!conn.connected());
        assert!(conn.name.is_none());
    }

    #[test]
    fn test_mud_connection_new_unique_ids() {
        let conn1 = MudConnection::new();
        let conn2 = MudConnection::new();
        assert_ne!(conn1.id, conn2.id);
    }

    #[test]
    fn test_mud_connection_not_connected() {
        let conn = MudConnection::new();
        assert!(!conn.connected());
    }

    #[test]
    fn test_mud_connection_disconnect_when_not_connected() {
        let mut conn = MudConnection::new();
        // Disconnect when not connected should be ok
        assert!(conn.disconnect().is_ok());
    }

    #[test]
    fn test_mud_connection_read_not_connected() {
        let mut conn = MudConnection::new();
        let mut buf = [0u8; 10];
        let result = conn.read(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_mud_connection_write_not_connected() {
        let mut conn = MudConnection::new();
        let result = conn.write(b"test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_mud_connection_flush_not_connected() {
        let mut conn = MudConnection::new();
        let result = conn.flush();
        assert!(result.is_ok());
    }

    #[test]
    fn test_mud_connection_write_all_not_connected() {
        let mut conn = MudConnection::new();
        let result = conn.write_all(b"test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mud_connection_clone() {
        let conn = MudConnection::new();
        let cloned = conn.clone();
        assert_eq!(conn.id, cloned.id);
        assert_eq!(conn.host, cloned.host);
        assert_eq!(conn.port, cloned.port);
    }

    #[test]
    fn test_connection_id_increments() {
        let id1 = connection_id();
        let id2 = connection_id();
        assert!(id2 > id1);
    }
}
