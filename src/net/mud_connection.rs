use anyhow::Result;
use lazy_static::lazy_static;
use log::debug;
use std::{
    io::Read,
    io::Write,
    net::Shutdown,
    net::TcpStream,
    sync::{atomic::AtomicU16, atomic::Ordering, Arc, Mutex},
};

use crate::net::open_tcp_stream;
use crate::net::tls::{CertificateValidation, TlsStream};

use super::RwStream;

#[derive(Clone)]
pub struct MudConnection {
    pub id: u16,
    stream: Option<RwStream<TcpStream>>,
    tls_stream: Option<TlsStream>,
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub tls_validation: CertificateValidation,
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
            tls_stream: None,
            host: "0.0.0.0".to_string(),
            port: 4000,
            tls: false,
            tls_validation: CertificateValidation::DangerousDisabled,
        }
    }

    fn get_input_stream(&self) -> Option<&Arc<Mutex<dyn Read + Send>>> {
        if let Some(stream) = &self.tls_stream {
            Some(&stream.input_stream)
        } else {
            self.stream.as_ref().map(|stream| &stream.input_stream)
        }
    }

    fn get_output_stream(&self) -> Option<&Arc<Mutex<dyn Write + Send>>> {
        if let Some(stream) = &self.tls_stream {
            Some(&stream.output_stream)
        } else {
            self.stream.as_ref().map(|stream| &stream.output_stream)
        }
    }

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

        debug!(
            "Connecting to {}:{} tls: {} verify: {}",
            host, port, tls, tls_validation
        );

        let stream = open_tcp_stream(&self.host, self.port)?;
        if tls {
            self.tls_stream = Some(TlsStream::tls_init(stream, host, tls_validation)?);
        } else {
            self.stream = Some(RwStream::new(stream));
        }
        self.id = connection_id();
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            debug!("Disconnecting from {}:{}", self.host, self.port);
            stream.inner().shutdown(Shutdown::Both)?;
            debug!("Disconnected from {}:{}", self.host, self.port);
            self.stream = None;
        } else if let Some(stream) = &self.tls_stream {
            debug!("Disconnecting from {}:{}", self.host, self.port);
            stream.inner_mut().conn.send_close_notify();
            stream.inner().sock.shutdown(Shutdown::Both)?;
            debug!("Disconnected from {}:{}", self.host, self.port);
            self.tls_stream = None;
        }
        Ok(())
    }

    pub fn connected(&self) -> bool {
        self.stream.is_some() || self.tls_stream.is_some()
    }
}

impl Read for MudConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut result = Ok(0);
        if let Some(stream) = &mut self.get_input_stream() {
            if let Ok(mut stream) = stream.lock() {
                result = stream.read(buf);
            }
        }
        result
    }
}

impl Write for MudConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut result = Ok(0);
        if let Some(stream) = &mut self.get_output_stream() {
            if let Ok(mut stream) = stream.lock() {
                result = stream.write(buf);
            }
        }
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut result = Ok(());
        if let Some(stream) = &mut self.get_output_stream() {
            if let Ok(mut stream) = stream.lock() {
                result = stream.flush();
            }
        }
        result
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        let mut result = Ok(());
        if let Some(stream) = &mut self.get_output_stream() {
            if let Ok(mut stream) = stream.lock() {
                result = stream.write_all(buf);
            }
        }
        result
    }
}
