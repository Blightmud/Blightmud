use libtelnet_rs::{compatibility::CompatibilityTable, telnet::op_option as opt, Parser};
use std::{
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        mpsc::Sender,
        Arc, Mutex,
    },
};

use crate::{io::Logger, lua::LuaScript, net::OutputBuffer, timer::TimerEvent, Event};
use log::debug;

#[derive(Default)]
pub struct CommunicationOptions {
    pub mccp2: bool,
    pub debug_gmcp: bool,
}

#[derive(Clone)]
pub struct Session {
    pub connection_id: u16,
    pub host: Arc<Mutex<String>>,
    pub port: Arc<AtomicU16>,
    pub connected: Arc<AtomicBool>,
    pub gmcp: Arc<AtomicBool>,
    pub stream: Arc<Mutex<Option<TcpStream>>>,
    pub main_writer: Sender<Event>,
    pub timer_writer: Sender<TimerEvent>,
    pub terminate: Arc<AtomicBool>,
    pub telnet_parser: Arc<Mutex<Parser>>,
    pub output_buffer: Arc<Mutex<OutputBuffer>>,
    pub prompt_input: Arc<Mutex<String>>,
    pub lua_script: Arc<Mutex<LuaScript>>,
    pub comops: Arc<Mutex<CommunicationOptions>>,
    pub logger: Arc<Mutex<Logger>>,
}

impl Session {
    pub fn connect(&mut self, host: &str, port: u16) -> bool {
        self.connection_id += 1;
        if let Ok(mut m_host) = self.host.lock() {
            *m_host = host.to_string();
        }
        self.port.store(port, Ordering::Relaxed);

        debug!("Connecting to {}:{}", host, port);
        if let Ok(stream) = TcpStream::connect(format!("{}:{}", host, port)) {
            self.stream.lock().unwrap().replace(stream);
            self.connected.store(true, Ordering::Relaxed);
            self.main_writer
                .send(Event::StartLogging(host.to_string(), false))
                .unwrap();
            self.main_writer.send(Event::Connected).unwrap();
        }
        self.connected.load(Ordering::Relaxed)
    }

    pub fn disconnect(&mut self) {
        if self.connected.load(Ordering::Relaxed) {
            let host = self.host.lock().unwrap();
            let port = self.port.load(Ordering::Relaxed);
            debug!("Disconnecting from {}:{}", host, port);
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
            self.stop_logging();
            debug!("Disconnected from {}:{}", host, port);
        }
    }

    pub fn start_logging(&self, host: &str) {
        if let Ok(mut logger) = self.logger.lock() {
            logger.start_logging(host).ok();
        }
    }

    pub fn stop_logging(&self) {
        if let Ok(mut logger) = self.logger.lock() {
            self.main_writer
                .send(Event::Info("Logging stopped".to_string()))
                .unwrap();
            logger.stop_logging().ok();
        }
    }

    pub fn send_event(&mut self, event: Event) {
        self.main_writer.send(event).unwrap();
    }

    pub fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.disconnect();
        self.main_writer.send(Event::Quit)?;
        self.timer_writer.send(TimerEvent::Quit)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct SessionBuilder {
    main_writer: Option<Sender<Event>>,
    timer_writer: Option<Sender<TimerEvent>>,
    screen_dimensions: Option<(u16, u16)>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self {
            main_writer: None,
            timer_writer: None,
            screen_dimensions: None,
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

    pub fn screen_dimensions(mut self, dimensions: (u16, u16)) -> Self {
        self.screen_dimensions = Some(dimensions);
        self
    }

    pub fn build(self) -> Session {
        let main_writer = self.main_writer.unwrap();
        let timer_writer = self.timer_writer.unwrap();
        let dimensions = self.screen_dimensions.unwrap();
        Session {
            connection_id: 0,
            host: Arc::new(Mutex::new(String::new())),
            port: Arc::new(AtomicU16::new(0)),
            connected: Arc::new(AtomicBool::new(false)),
            gmcp: Arc::new(AtomicBool::new(false)),
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
            lua_script: Arc::new(Mutex::new(LuaScript::new(main_writer, dimensions))),
            comops: Arc::new(Mutex::new(CommunicationOptions::default())),
            logger: Arc::new(Mutex::new(Logger::default())),
        }
    }
}

fn build_compatibility_table() -> CompatibilityTable {
    let mut telnet_compat = CompatibilityTable::default();
    telnet_compat.support(opt::MCCP2);
    telnet_compat.support(opt::GMCP);
    telnet_compat.support(opt::EOR);
    telnet_compat.support(opt::ECHO);
    telnet_compat
}

#[cfg(test)]
mod session_test {

    use super::{Session, SessionBuilder};
    use crate::{event::Event, model::Line, timer::TimerEvent};
    use std::sync::mpsc::{channel, Receiver, Sender};

    fn build_session() -> (Session, Receiver<Event>, Receiver<TimerEvent>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let (timer_writer, timer_reader): (Sender<TimerEvent>, Receiver<TimerEvent>) = channel();
        let session = SessionBuilder::new()
            .main_writer(writer.clone())
            .timer_writer(timer_writer.clone())
            .screen_dimensions((80, 80))
            .build();

        (session, reader, timer_reader)
    }

    #[test]
    fn test_session_build() {
        let _ = build_session();
    }

    #[test]
    fn test_session_send_event() {
        let (mut session, reader, _timer_reader) = build_session();
        session.send_event(Event::Output(Line::from("test test")));
        assert_eq!(reader.recv(), Ok(Event::Output(Line::from("test test"))));
    }

    #[test]
    fn test_logging() {
        let (session, reader, _timer_reader) = build_session();
        assert!(!session.logger.lock().unwrap().is_logging());
        session.start_logging("mysteryhost");
        assert!(session.logger.lock().unwrap().is_logging());
        session.stop_logging();
        assert_eq!(
            reader.recv(),
            Ok(Event::Info("Logging stopped".to_string()))
        );
        assert!(!session.logger.lock().unwrap().is_logging());
    }

    #[test]
    fn test_close() {
        let (mut session, reader, timer_reader) = build_session();
        session.close().unwrap();
        assert_eq!(reader.recv(), Ok(Event::Quit));
        assert_eq!(timer_reader.recv(), Ok(TimerEvent::Quit));
    }
}
