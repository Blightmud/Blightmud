use crate::{
    model::{Connection, Line},
    net::{spawn_receive_thread, spawn_transmit_thread},
    session::Session,
    tts::TTSEvent,
    ui::UserInterface,
    TelnetData,
};
use libtelnet_rs::events::TelnetEvents;
use log::debug;
use std::{
    error::Error,
    sync::mpsc::{channel, Receiver, Sender},
};

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    Prompt(Line),
    ServerSend(Vec<u8>),
    InputSent(Line),
    ServerInput(Line),
    MudOutput(Line),
    Output(Line),
    Error(String),
    Info(String),
    UserInputBuffer(String, usize),
    Connect(Connection),
    Connected(u16),
    Disconnect(u16),
    Reconnect,
    ProtoEnabled(u8),
    EnableProto(u8),
    DisableProto(u8),
    ProtoSubnegRecv(u8, Vec<u8>),
    ProtoSubnegSend(u8, Vec<u8>),
    AddTimedEvent(chrono::Duration, Option<u32>, u32, bool),
    TimedEvent(u32),
    DropTimedEvent(u32),
    ClearTimers,
    RemoveTimer(u32),
    LoadScript(String),
    ResetScript,
    StartLogging(String, bool),
    StopLogging,
    SettingChanged(String, bool),
    ScrollLock(bool),
    ScrollUp,
    ScrollDown,
    ScrollTop,
    ScrollBottom,
    StatusAreaHeight(u16),
    StatusLine(usize, String),
    ShowHelp(String, bool),
    TTSEnabled(bool),
    Speak(String, bool),
    SpeakStop,
    TTSEvent(TTSEvent),
    PlayMusic(String, bool),
    StopMusic,
    PlaySFX(String),
    StopSFX,
    Redraw,
    Quit,
}

type Result = std::result::Result<(), Box<dyn Error>>;

pub struct EventHandler {
    session: Session,
}

impl From<&Session> for EventHandler {
    fn from(session: &Session) -> Self {
        Self {
            session: session.clone(),
        }
    }
}

pub struct BadEventRoutingError;

impl std::fmt::Debug for BadEventRoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad Event routing")
    }
}
impl std::fmt::Display for BadEventRoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bad Event routing")
    }
}

impl Error for BadEventRoutingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl EventHandler {
    pub fn handle_server_events(
        &mut self,
        event: Event,
        screen: &mut dyn UserInterface,
        transmit_writer: &mut Option<Sender<TelnetData>>,
    ) -> Result {
        match event {
            Event::ServerSend(data) => {
                debug!("Sending: {:?}", data);
                if let Some(transmit_writer) = &transmit_writer {
                    transmit_writer.send(Some(data))?;
                } else {
                    screen.print_error("No active session");
                }
                Ok(())
            }
            Event::ServerInput(mut line) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    let mut output_buffer = self.session.output_buffer.lock().unwrap();
                    output_buffer.input_sent();
                    script.on_mud_input(&mut line);
                    screen.print_send(&line);
                    if let Ok(mut logger) = self.session.logger.lock() {
                        logger.log_line("> ", &line)?;
                    }
                    if !line.flags.matched {
                        if let Ok(mut parser) = self.session.telnet_parser.lock() {
                            if let TelnetEvents::DataSend(buffer) = parser.send_text(&line.line()) {
                                self.session.main_writer.send(Event::ServerSend(buffer))?;
                            }
                        }
                    }
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                Ok(())
            }
            Event::Connect(Connection { host, port, tls }) => {
                self.session.disconnect();
                if self.session.connect(&host, port, tls) {
                    let (writer, reader): (Sender<TelnetData>, Receiver<TelnetData>) = channel();
                    spawn_receive_thread(self.session.clone());
                    spawn_transmit_thread(self.session.clone(), reader);
                    transmit_writer.replace(writer);
                } else {
                    screen.print_error(&format!("Failed to connect to {}:{}", host, port));
                }
                Ok(())
            }
            Event::Connected(id) => {
                let host = self.session.host();
                let port = self.session.port();
                debug!("Connected to {}:{}", host, port);
                screen.set_host(&host, port)?;
                if let Ok(mut script) = self.session.lua_script.lock() {
                    script.on_connect(&host, port, id);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                Ok(())
            }
            Event::Disconnect(id) => {
                if self.session.connection_id() == id && self.session.connected() {
                    self.session.disconnect();
                    screen.print_info(&format!(
                        "Disconnecting from: {}:{}",
                        self.session.host(),
                        self.session.port()
                    ));
                    if let Some(transmit_writer) = &transmit_writer {
                        transmit_writer.send(None)?;
                    }
                    if let Ok(mut script) = self.session.lua_script.lock() {
                        script.on_disconnect();
                        script.get_output_lines().iter().for_each(|l| {
                            screen.print_output(l);
                        });
                    }
                    transmit_writer.take();
                    screen.set_host("", 0)?;
                }
                Ok(())
            }
            Event::Reconnect => {
                let host = self.session.host();
                let port = self.session.port();
                let tls = self.session.tls();
                if !host.is_empty() && !port > 0 {
                    self.session
                        .main_writer
                        .send(Event::Connect(Connection::new(&host, port, tls)))?;
                } else {
                    screen.print_error("Reconnect to what?");
                }
                Ok(())
            }
            _ => Err(BadEventRoutingError.into()),
        }
    }

    fn log_line(&self, prefix: &str, line: &Line) -> Result {
        if let Ok(mut logger) = self.session.logger.lock() {
            logger.log_line(prefix, line)?;
        }
        Ok(())
    }

    fn log_str(&self, prefix: &str, line: &str) -> Result {
        if let Ok(mut logger) = self.session.logger.lock() {
            logger.log_str(&format!("{}{}", prefix, line))?;
        }
        Ok(())
    }

    fn handle_logging(&self, event: Event) -> Result {
        match event {
            Event::MudOutput(line) | Event::Output(line) => self.log_line("", &line),
            Event::Error(line) => self.log_str("[!!] ", &line),
            Event::Info(line) => self.log_str("[**] ", &line),
            Event::Prompt(prompt) => {
                self.log_line("", &prompt)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn handle_output_events(&self, event: Event, screen: &mut dyn UserInterface) -> Result {
        self.handle_logging(event.clone())?;
        match event {
            Event::MudOutput(mut line) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    script.on_mud_output(&mut line);
                    screen.print_output(&line);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                Ok(())
            }
            Event::Output(line) => {
                screen.print_output(&line);
                Ok(())
            }
            Event::Prompt(mut prompt) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    script.on_mud_output(&mut prompt);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                screen.print_prompt(&prompt);
                Ok(())
            }
            Event::UserInputBuffer(input_buffer, pos) => {
                let mut prompt_input = self.session.prompt_input.lock().unwrap();
                *prompt_input = input_buffer;
                screen.print_prompt_input(&prompt_input, pos);
                Ok(())
            }
            Event::Error(msg) => {
                screen.print_error(&msg);
                Ok(())
            }
            Event::Info(msg) => {
                screen.print_info(&msg);
                Ok(())
            }
            Event::InputSent(msg) => {
                let mut output_buffer = self.session.output_buffer.lock().unwrap();
                output_buffer.input_sent();
                screen.print_send(&msg);
                Ok(())
            }
            _ => Err(BadEventRoutingError.into()),
        }
    }
}

#[cfg(test)]
mod event_test {

    use std::sync::{Arc, Mutex};

    use mockall::predicate::eq;

    use crate::{session::SessionBuilder, timer::TimerEvent};

    use crate::io::MockLogWriter;
    use crate::ui::MockUserInterface;

    use super::*;

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
    fn test_event_logging() {
        let (mut session, _reader, _timer_reader) = build_session();
        let mut logger = MockLogWriter::new();
        logger
            .expect_log_str()
            .with(eq("prefix test line"))
            .returning(|_| Ok(()));
        logger
            .expect_log_line()
            .with(eq("prefix "), eq(Line::from("test line")))
            .returning(|_, _| Ok(()));
        session.logger = Arc::new(Mutex::new(logger));
        let handler = EventHandler::from(&session);
        let _ = handler.log_str("prefix ", "test line");
        let _ = handler.log_line("prefix ", &Line::from("test line"));
    }

    #[test]
    fn test_output() {
        let (mut session, _reader, _timer_reader) = build_session();
        let mut logger = MockLogWriter::new();
        logger.expect_log_line().times(3).returning(|_, _| Ok(()));
        logger.expect_log_str().times(2).returning(|_| Ok(()));
        session.logger = Arc::new(Mutex::new(logger));
        let handler = EventHandler::from(&session);

        let mut screen = MockUserInterface::new();
        screen
            .expect_print_output()
            .with(eq(Line::from("Output line")))
            .times(2)
            .return_const(());
        screen.expect_print_prompt().times(1).return_const(());
        screen.expect_print_prompt_input().times(1).return_const(());
        screen.expect_print_error().times(1).return_const(());
        screen.expect_print_info().times(1).return_const(());
        screen
            .expect_print_send()
            .with(eq(Line::from("input data")))
            .times(1)
            .return_const(());

        let line = Line::from("Output line");
        assert!(handler
            .handle_output_events(Event::MudOutput(line.clone()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Output(line.clone()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Prompt(Line::from("")), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(
                Event::UserInputBuffer(String::from("prompt"), 5),
                &mut screen
            )
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Info("info message".to_string()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Error("error message".to_string()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(Event::InputSent(Line::from("input data")), &mut screen)
            .is_ok());
    }
}
