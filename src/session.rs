use libtelnet_rs::Parser;
use std::{
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, Mutex,
    },
};

use crate::Event;
use log::debug;

#[derive(Clone)]
pub struct Session {
    pub host: String,
    pub port: u32,
    pub connected: Arc<AtomicBool>,
    pub stream: Arc<Mutex<Option<TcpStream>>>,
    pub main_thread_writer: Sender<Event>,
    pub terminate: Arc<AtomicBool>,
    pub telnet_parser: Arc<Mutex<Parser>>,
}

impl Session {
    pub fn connect(&mut self, host: &str, port: u32) -> bool {
        self.host = host.to_string();
        self.port = port;
        debug!("Connecting to {}:{}", self.host, self.port);
        if let Ok(stream) = TcpStream::connect(format!("{}:{}", host, port)) {
            self.stream.lock().unwrap().replace(stream);
            self.connected.store(true, Ordering::Relaxed);
            debug!("Connected to {}:{}", self.host, self.port);
        }
        self.connected.load(Ordering::Relaxed)
    }

    pub fn disconnect(&mut self) {
        debug!("Disconnecting from {}:{}", self.host, self.port);
        if self.connected.load(Ordering::Relaxed) {
            if let Ok(mut stream) = self.stream.lock() {
                stream
                    .as_mut()
                    .unwrap()
                    .shutdown(std::net::Shutdown::Both)
                    .ok();
                *stream = None;
                self.connected.store(false, Ordering::Relaxed);
            }
        }
        debug!("Disconnected from {}:{}", self.host, self.port);
    }

    pub fn send_event(&mut self, event: Event) {
        self.main_thread_writer.send(event).unwrap();
    }

    pub fn close(&mut self) {
        if self.connected.load(Ordering::Relaxed) {
            self.disconnect();
        }
        self.main_thread_writer.send(Event::Quit).unwrap();
    }
}

#[derive(Clone)]
pub struct SessionBuilder {
    main_thread_writer: Option<Sender<Event>>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self {
            main_thread_writer: None,
        }
    }

    pub fn main_thread_writer(mut self, main_thread_writer: Sender<Event>) -> Self {
        self.main_thread_writer = Some(main_thread_writer);
        self
    }

    pub fn build(self) -> Session {
        Session {
            host: String::new(),
            port: 0,
            connected: Arc::new(AtomicBool::new(false)),
            stream: Arc::new(Mutex::new(None)),
            main_thread_writer: self.main_thread_writer.unwrap(),
            terminate: Arc::new(AtomicBool::new(false)),
            telnet_parser: Arc::new(Mutex::new(Parser::with_capacity(1024))),
        }
    }
}
