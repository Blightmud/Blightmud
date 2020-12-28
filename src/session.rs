use libtelnet_rs::{compatibility::CompatibilityTable, telnet::op_option as opt, Parser};
use log::debug;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc, Mutex,
};

use crate::{
    io::{LogWriter, Logger},
    lua::LuaScript,
    net::MudConnection,
    net::BUFFER_SIZE,
    net::{OutputBuffer, TelnetMode},
    timer::TimerEvent,
    tts::TTSController,
    Event,
};

#[cfg(test)]
use mockall::automock;

#[derive(Clone)]
pub struct Session {
    pub connection: Arc<Mutex<MudConnection>>,
    pub gmcp: Arc<AtomicBool>,
    pub main_writer: Sender<Event>,
    pub timer_writer: Sender<TimerEvent>,
    pub telnet_parser: Arc<Mutex<Parser>>,
    pub output_buffer: Arc<Mutex<OutputBuffer>>,
    pub prompt_input: Arc<Mutex<String>>,
    pub save_history: Arc<AtomicBool>,
    pub lua_script: Arc<Mutex<LuaScript>>,
    pub logger: Arc<Mutex<dyn LogWriter + Send>>,
    pub tts_ctrl: Arc<Mutex<TTSController>>,
}

#[cfg_attr(test, automock)]
impl Session {
    pub fn connect(&mut self, host: &str, port: u16, tls: bool) -> bool {
        let mut connected = false;
        if let Ok(mut connection) = self.connection.lock() {
            connected = match connection.connect(host, port, tls) {
                Ok(_) => true,
                Err(err) => {
                    debug!("Failed to connect: {}", err);
                    false
                }
            };
        }
        if connected {
            self.main_writer
                .send(Event::StartLogging(host.to_string(), false))
                .unwrap();
            self.main_writer.send(Event::Connected).unwrap();
        }
        connected
    }

    pub fn disconnect(&mut self) {
        let mut connection = self.connection.lock().unwrap();
        if connection.connected() {
            connection.disconnect().ok();
            if let Ok(mut output_buffer) = self.output_buffer.lock() {
                output_buffer.clear()
            }

            if let Ok(mut parser) = self.telnet_parser.lock() {
                parser.options.reset_states();
            };

            self.stop_logging();
        }
    }

    pub fn connected(&self) -> bool {
        let connection = self.connection.lock().unwrap();
        connection.connected()
    }

    pub fn connection_id(&self) -> u16 {
        let connection = self.connection.lock().unwrap();
        connection.id
    }

    pub fn host(&self) -> String {
        let connection = self.connection.lock().unwrap();
        connection.host.clone()
    }

    pub fn port(&self) -> u16 {
        let connection = self.connection.lock().unwrap();
        connection.port
    }

    pub fn tls(&self) -> bool {
        let connection = self.connection.lock().unwrap();
        connection.tls
    }

    pub fn start_logging(&self, host: &str) {
        if let Ok(mut logger) = self.logger.lock() {
            self.main_writer
                .send(Event::Info(format!("Started logging for: {}", host)))
                .unwrap();
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

    pub fn save_history(&self) -> bool {
        self.save_history.load(Ordering::Relaxed)
    }

    pub fn set_save_history(&self, toggle: bool) {
        self.save_history.store(toggle, Ordering::Relaxed);
    }

    pub fn send_event(&mut self, event: Event) {
        self.main_writer.send(event).unwrap();
    }

    pub fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.disconnect();
        self.main_writer.send(Event::Quit)?;
        self.timer_writer.send(TimerEvent::Quit)?;
        self.tts_ctrl.lock().unwrap().shutdown();
        Ok(())
    }
}

#[derive(Clone)]
pub struct SessionBuilder {
    main_writer: Option<Sender<Event>>,
    timer_writer: Option<Sender<TimerEvent>>,
    screen_dimensions: Option<(u16, u16)>,
    tts_enabled: bool,
    save_history: bool,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self {
            main_writer: None,
            timer_writer: None,
            screen_dimensions: None,
            tts_enabled: false,
            save_history: false,
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

    pub fn tts_enabled(mut self, enabled: bool) -> Self {
        self.tts_enabled = enabled;
        self
    }

    pub fn save_history(mut self, enabled: bool) -> Self {
        self.save_history = enabled;
        self
    }

    pub fn build(self) -> Session {
        let main_writer = self.main_writer.unwrap();
        let timer_writer = self.timer_writer.unwrap();
        let dimensions = self.screen_dimensions.unwrap();
        let tts_enabled = self.tts_enabled;
        let save_history = self.save_history;
        Session {
            connection: Arc::new(Mutex::new(MudConnection::new())),
            gmcp: Arc::new(AtomicBool::new(false)),
            main_writer: main_writer.clone(),
            timer_writer,
            telnet_parser: Arc::new(Mutex::new(Parser::with_support_and_capacity(
                BUFFER_SIZE,
                build_compatibility_table(),
            ))),
            output_buffer: Arc::new(Mutex::new(OutputBuffer::new(
                &TelnetMode::UnterminatedPrompt,
            ))),
            prompt_input: Arc::new(Mutex::new(String::new())),
            save_history: Arc::new(AtomicBool::new(save_history)),
            lua_script: Arc::new(Mutex::new(LuaScript::new(main_writer, dimensions))),
            logger: Arc::new(Mutex::new(Logger::default())),
            tts_ctrl: Arc::new(Mutex::new(TTSController::new(tts_enabled))),
        }
    }
}

fn build_compatibility_table() -> CompatibilityTable {
    let mut telnet_compat = CompatibilityTable::default();
    telnet_compat.support(opt::MCCP2);
    telnet_compat.support(opt::EOR);
    telnet_compat.support(opt::ECHO);
    telnet_compat
}

#[cfg(test)]
mod session_test {

    use mockall::predicate::eq;

    use super::*;
    use crate::io::MockLogWriter;
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

        loop {
            if reader.try_recv().is_err() {
                break;
            }
        }

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
        let (mut session, reader, _timer_reader) = build_session();
        let mut logger = MockLogWriter::new();
        logger
            .expect_start_logging()
            .with(eq("mysteryhost"))
            .times(1)
            .returning(|_| Ok(()));
        logger.expect_stop_logging().times(1).returning(|| Ok(()));
        session.logger = Arc::new(Mutex::new(logger));

        session.start_logging("mysteryhost");
        assert_eq!(
            reader.recv(),
            Ok(Event::Info("Started logging for: mysteryhost".to_string()))
        );
        session.stop_logging();
        assert_eq!(
            reader.recv(),
            Ok(Event::Info("Logging stopped".to_string()))
        );
    }

    #[test]
    fn test_close() {
        let (mut session, reader, timer_reader) = build_session();
        session.close().unwrap();
        assert_eq!(reader.recv(), Ok(Event::Quit));
        assert_eq!(timer_reader.recv(), Ok(TimerEvent::Quit));
    }
}
