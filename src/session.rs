use std::{
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
};

use crate::TelnetData;

pub struct Session {
    pub host: String,
    pub port: u32,
    pub connected: Arc<AtomicBool>,
    pub stream: Option<TcpStream>,
    pub terminate: Arc<AtomicBool>,
    pub transmit_writer: Sender<TelnetData>,
    pub output_writer: Sender<TelnetData>,
    pub local_output_writer: Sender<String>,
    pub input_writer: Sender<Option<String>>,
    pub ui_update_notifier: Sender<bool>,
    pub input_buffer_write: Sender<String>,
}

impl Clone for Session {
    fn clone(&self) -> Self {
        let stream = match &self.stream {
            Some(stream) => Some(stream.try_clone().unwrap()),
            _ => None,
        };

        Self {
            host: self.host.clone(),
            port: self.port,
            connected: self.connected.clone(),
            stream,
            terminate: self.terminate.clone(),
            transmit_writer: self.transmit_writer.clone(),
            output_writer: self.output_writer.clone(),
            local_output_writer: self.local_output_writer.clone(),
            input_writer: self.input_writer.clone(),
            ui_update_notifier: self.ui_update_notifier.clone(),
            input_buffer_write: self.input_buffer_write.clone(),
        }
    }
}

impl Session {
    pub fn _connect(&mut self, host: &str, port: u32) -> bool {
        self.host = host.to_string();
        self.port = port;
        if let Ok(stream) = TcpStream::connect(format!("{}:{}", host, port)) {
            self.stream = Some(stream);
            self.connected.store(true, Ordering::Relaxed);
        }
        self.connected.load(Ordering::Relaxed)
    }

    pub fn close(&mut self) {
        if let Some(stream) = &self.stream {
            stream.shutdown(std::net::Shutdown::Both).ok();
        }
        self.terminate.store(true, Ordering::Relaxed);
        self.transmit_writer.send(None).unwrap();
        self.output_writer.send(None).unwrap();
        self.input_writer.send(None).unwrap();
        self.ui_update_notifier.send(true).unwrap();
    }
}

#[derive(Clone)]
pub struct SessionBuilder {
    terminate: Arc<AtomicBool>,
    transmit_writer: Option<Sender<TelnetData>>,
    output_writer: Option<Sender<TelnetData>>,
    local_output_writer: Option<Sender<String>>,
    input_writer: Option<Sender<Option<String>>>,
    ui_update_notifier: Option<Sender<bool>>,
    input_buffer_write: Option<Sender<String>>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self {
            terminate: Arc::new(AtomicBool::new(false)),
            transmit_writer: None,
            output_writer: None,
            local_output_writer: None,
            input_writer: None,
            ui_update_notifier: None,
            input_buffer_write: None,
        }
    }

    pub fn transmit_writer(mut self, transmit_writer: Sender<TelnetData>) -> Self {
        self.transmit_writer = Some(transmit_writer);
        self
    }

    pub fn output_writer(mut self, output_writer: Sender<TelnetData>) -> Self {
        self.output_writer = Some(output_writer);
        self
    }

    pub fn local_output_writer(mut self, local_output_writer: Sender<String>) -> Self {
        self.local_output_writer = Some(local_output_writer);
        self
    }

    pub fn input_writer(mut self, input_writer: Sender<Option<String>>) -> Self {
        self.input_writer = Some(input_writer);
        self
    }

    pub fn ui_update_notifier(mut self, ui_update_notifier: Sender<bool>) -> Self {
        self.ui_update_notifier = Some(ui_update_notifier);
        self
    }

    pub fn input_buffer_write(mut self, input_buffer_write: Sender<String>) -> Self {
        self.input_buffer_write = Some(input_buffer_write);
        self
    }

    pub fn build(self) -> Session {
        Session {
            host: String::new(),
            port: 0,
            connected: Arc::new(AtomicBool::new(false)),
            stream: None,
            terminate: self.terminate,
            transmit_writer: self.transmit_writer.unwrap(),
            output_writer: self.output_writer.unwrap(),
            local_output_writer: self.local_output_writer.unwrap(),
            input_writer: self.input_writer.unwrap(),
            ui_update_notifier: self.ui_update_notifier.unwrap(),
            input_buffer_write: self.input_buffer_write.unwrap(),
        }
    }
}
