use anyhow::{bail, Result};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

#[derive(Default)]
pub struct Connection {
    pub stream: Option<TcpStream>,
    pub buffer: Arc<Mutex<Vec<u8>>>,
}

#[allow(dead_code)]
impl Connection {
    pub fn recv(&mut self) -> Vec<u8> {
        if let Some(stream) = self.stream.as_mut() {
            let mut buffer = vec![0u8; 1024];
            if let Ok(count) = stream.read(&mut buffer) {
                buffer[..count].to_vec()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    pub fn recv_string(&mut self) -> String {
        let data = self.recv();
        String::from_utf8_lossy(&data).to_owned().to_string()
    }

    pub fn send(&mut self, bytes: &[u8]) {
        if let Some(stream) = self.stream.as_mut() {
            let _ = stream.write(bytes);
        }
    }

    pub fn close(mut self) {
        if let Some(stream) = self.stream.as_mut() {
            stream.shutdown(std::net::Shutdown::Both).ok();
        }
    }
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        let stream = if let Some(stream) = &self.stream {
            stream.try_clone().ok()
        } else {
            None
        };
        Self {
            stream,
            buffer: self.buffer.clone(),
        }
    }
}

pub struct Server {
    connection_receiver: Receiver<Connection>,
}

impl Server {
    pub fn bind(port: u16) -> Self {
        let (tx, rx): (Sender<Connection>, Receiver<Connection>) = channel();
        let server = Self {
            connection_receiver: rx,
        };
        spawn_listener_thread(tx, port);
        server
    }

    pub fn listen(&mut self) -> Result<Connection> {
        if let Ok(connection) = self.connection_receiver.recv() {
            Ok(connection)
        } else {
            bail!("Failed to get connection")
        }
    }
}

fn spawn_listener_thread(tx: Sender<Connection>, port: u16) {
    thread::spawn(move || -> Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                tx.send(Connection {
                    stream: Some(stream),
                    buffer: Arc::new(Mutex::new(Vec::new())),
                })
                .unwrap();
            }
        }
        Ok(())
    });
}
