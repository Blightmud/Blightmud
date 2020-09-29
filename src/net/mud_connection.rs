use anyhow::Result;
use lazy_static::lazy_static;
use log::debug;
use std::{
    io::Read,
    io::Write,
    net::Shutdown,
    net::TcpStream,
    sync::{atomic::AtomicU16, atomic::Ordering},
};

use super::RwStream;

#[derive(Clone)]
pub struct MudConnection {
    pub id: u16,
    stream: Option<RwStream<TcpStream>>,
    pub host: String,
    pub port: u16,
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
        }
    }

    pub fn connect(&mut self, host: &str, port: u16) -> Result<()> {
        self.host = host.to_string();
        self.port = port;

        let uri = format!("{}:{}", self.host, self.port);
        debug!("Connecting to {}:{}", host, port);
        self.stream = Some(RwStream::new(TcpStream::connect(uri)?));
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            debug!("Disconnecting from {}:{}", self.host, self.port);
            stream.inner().shutdown(Shutdown::Both)?;
            debug!("Disconnected from {}:{}", self.host, self.port);
            self.stream = None;
        }
        Ok(())
    }

    pub fn connected(&self) -> bool {
        self.stream.is_some()
    }
}

impl Read for MudConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut result = Ok(0);
        if let Some(stream) = &mut self.stream {
            if let Ok(mut stream) = stream.input_stream.lock() {
                result = stream.read(buf);
            }
        }
        result
    }
}

impl Write for MudConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut result = Ok(0);
        if let Some(stream) = &mut self.stream {
            if let Ok(mut stream) = stream.output_stream.lock() {
                result = stream.write(buf);
            }
        }
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut result = Ok(());
        if let Some(stream) = &mut self.stream {
            if let Ok(mut stream) = stream.output_stream.lock() {
                result = stream.flush();
            }
        }
        result
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let mut result = Ok(());
        if let Some(stream) = &mut self.stream {
            if let Ok(mut stream) = stream.output_stream.lock() {
                result = stream.write_all(buf);
            }
        }
        result
    }
}
