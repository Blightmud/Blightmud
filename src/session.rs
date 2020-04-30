use std::{
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, Mutex,
    },
};

use crate::Event;
use crate::TelnetData;

#[derive(Clone)]
pub struct Session {
    pub host: String,
    pub port: u32,
    pub connected: Arc<AtomicBool>,
    pub stream: Arc<Mutex<Option<TcpStream>>>,
    pub transmit_writer: Sender<TelnetData>,
    pub main_thread_writer: Sender<Event>,
    pub terminate: Arc<AtomicBool>,
}

impl Session {
    pub fn _connect(&mut self, host: &str, port: u32) -> bool {
        self.host = host.to_string();
        self.port = port;
        if let Ok(stream) = TcpStream::connect(format!("{}:{}", host, port)) {
            self.stream.lock().unwrap().replace(stream);
            self.connected.store(true, Ordering::Relaxed);
        }
        self.connected.load(Ordering::Relaxed)
    }

    pub fn disconnect(&mut self) {
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

    pub fn close(&mut self) {
        if self.connected.load(Ordering::Relaxed) {
            self.disconnect();
        }
        self.transmit_writer.send(None).unwrap();
        self.main_thread_writer.send(Event::Quit).unwrap();
    }
}

#[derive(Clone)]
pub struct SessionBuilder {
    transmit_writer: Option<Sender<TelnetData>>,
    main_thread_writer: Option<Sender<Event>>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self {
            transmit_writer: None,
            main_thread_writer: None,
        }
    }

    pub fn transmit_writer(mut self, transmit_writer: Sender<TelnetData>) -> Self {
        self.transmit_writer = Some(transmit_writer);
        self
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
            transmit_writer: self.transmit_writer.unwrap(),
            main_thread_writer: self.main_thread_writer.unwrap(),
            terminate: Arc::new(AtomicBool::new(false)),
        }
    }
}
