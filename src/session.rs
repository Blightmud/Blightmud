use libtelnet_rs::{compatibility::CompatibilityTable, telnet::op_option as opt, Parser};
use std::{
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc, Mutex,
    },
};

use crate::{lua::LuaScript, output_buffer::OutputBuffer, timer::TimerEvent, Event};
use log::debug;

#[derive(Default)]
pub struct CommunicationOptions {
    pub mccp2: bool,
}

#[derive(Clone)]
pub struct Session {
    pub host: String,
    pub port: u16,
    pub connected: Arc<AtomicBool>,
    pub stream: Arc<Mutex<Option<TcpStream>>>,
    pub main_writer: Sender<Event>,
    pub timer_writer: Sender<TimerEvent>,
    pub terminate: Arc<AtomicBool>,
    pub telnet_parser: Arc<Mutex<Parser>>,
    pub output_buffer: Arc<Mutex<OutputBuffer>>,
    pub prompt_input: Arc<Mutex<String>>,
    pub lua_script: Arc<Mutex<LuaScript>>,
    pub comops: Arc<Mutex<CommunicationOptions>>,
}

impl Session {
    pub fn connect(&mut self, host: &str, port: u16) -> bool {
        self.host = host.to_string();
        self.port = port;
        debug!("Connecting to {}:{}", self.host, self.port);
        if let Ok(stream) = TcpStream::connect(format!("{}:{}", host, port)) {
            self.stream.lock().unwrap().replace(stream);
            self.connected.store(true, Ordering::Relaxed);
            self.main_writer.send(Event::Connected).unwrap();
        }
        self.connected.load(Ordering::Relaxed)
    }

    pub fn disconnect(&mut self) {
        if self.connected.load(Ordering::Relaxed) {
            debug!("Disconnecting from {}:{}", self.host, self.port);
            if let Ok(mut stream) = self.stream.lock() {
                stream
                    .as_mut()
                    .unwrap()
                    .shutdown(std::net::Shutdown::Both)
                    .ok();
                *stream = None;
                self.connected.store(false, Ordering::Relaxed);
            }
            if let Ok(mut output_buffer) = self.output_buffer.lock() {
                output_buffer.clear()
            }
            self.comops = Arc::new(Mutex::new(CommunicationOptions::default()));
            self.telnet_parser = Arc::new(Mutex::new(Parser::with_support_and_capacity(
                4096,
                build_compatibility_table(),
            )));
            debug!("Disconnected from {}:{}", self.host, self.port);
        }
    }

    pub fn send_event(&mut self, event: Event) {
        self.main_writer.send(event).unwrap();
    }

    pub fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.connected.load(Ordering::Relaxed) {
            self.disconnect();
        }
        self.main_writer.send(Event::Quit)?;
        self.timer_writer.send(TimerEvent::Quit)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct SessionBuilder {
    main_writer: Option<Sender<Event>>,
    timer_writer: Option<Sender<TimerEvent>>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self {
            main_writer: None,
            timer_writer: None,
        }
    }

    pub fn main_writer(mut self, main_writer: Sender<Event>) -> Self {
        self.main_writer = Some(main_writer);
        self
    }

    pub fn timer_writer(mut self, timer_writer: Sender<TimerEvent>) -> Self {
        self.timer_writer = Some(timer_writer);
        self
    }

    pub fn build(self) -> Session {
        let main_writer = self.main_writer.unwrap();
        let timer_writer = self.timer_writer.unwrap();
        Session {
            host: String::new(),
            port: 0,
            connected: Arc::new(AtomicBool::new(false)),
            stream: Arc::new(Mutex::new(None)),
            main_writer: main_writer.clone(),
            timer_writer,
            terminate: Arc::new(AtomicBool::new(false)),
            telnet_parser: Arc::new(Mutex::new(Parser::with_support_and_capacity(
                4096,
                build_compatibility_table(),
            ))),
            output_buffer: Arc::new(Mutex::new(OutputBuffer::new())),
            prompt_input: Arc::new(Mutex::new(String::new())),
            lua_script: Arc::new(Mutex::new(LuaScript::new(main_writer))),
            comops: Arc::new(Mutex::new(CommunicationOptions::default())),
        }
    }
}

fn build_compatibility_table() -> CompatibilityTable {
    let mut telnet_compat = CompatibilityTable::default();
    telnet_compat.support(opt::MCCP2);
    telnet_compat.support(opt::GMCP);
    telnet_compat
}
