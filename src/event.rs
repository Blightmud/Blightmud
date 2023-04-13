use crate::io::FSEvent;
use crate::net::spawn_connect_thread;
use crate::{audio::SourceOptions, model::Regex};
use crate::{
    model::{Connection, Line, PromptMask},
    net::{spawn_receive_thread, spawn_transmit_thread},
    session::Session,
    tts::TTSEvent,
    ui::UserInterface,
    TelnetData,
};
use libtelnet_rs::{bytes::Bytes, events::TelnetEvents};
use log::debug;
use std::thread::JoinHandle;
use std::{
    error::Error,
    sync::mpsc::{channel, Receiver, Sender},
    thread, time,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum QuitMethod {
    CtrlC,
    Script,
    System,
    Error(String),
}

#[derive(Debug, PartialEq, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Event {
    AddTag(String),
    AddTimedEvent(chrono::Duration, Option<u32>, u32, bool),
    ClearTags,
    ClearTimers,
    Connect(Connection),
    Connected(u16),
    DisableProto(u8),
    Disconnect,
    DropTimedEvent(u32),
    EnableProto(u8),
    Error(String),
    FindBackward(Regex),
    FindForward(Regex),
    Info(String),
    LoadScript(String),
    EvalScript(String),
    MudOutput(Line),
    Output(Line),
    PlayMusic(String, SourceOptions),
    PlaySFX(String, SourceOptions),
    Prompt(Line),
    ProtoEnabled(u8),
    ProtoSubnegRecv(u8, Bytes),
    ProtoSubnegSend(u8, Bytes),
    Quit(QuitMethod),
    QuitConfirmTimeout,
    Reconnect,
    Redraw,
    RemoveTimer(u32),
    ResetScript,
    ScrollBottom,
    ScrollDown,
    ScrollLock(bool),
    ScrollTop,
    ScrollUp,
    ServerInput(Line),
    ServerSend(Bytes),
    SettingChanged(String, bool),
    ShowHelp(String, bool),
    Speak(String, bool),
    SpeakStop,
    StartLogging(String, bool),
    StatusAreaHeight(u16),
    StatusLine(usize, String),
    StopLogging,
    StopMusic,
    StopSFX,
    TTSEnabled(bool),
    TTSEvent(TTSEvent),
    TimedEvent(u32),
    TimerTick(u128),
    SetPromptInput(String),
    SetPromptCursorPos(usize),
    SetPromptMask(PromptMask),
    ClearPromptMask,
    UserInputBuffer(String, usize),
    UserInputCursor(usize),
    FSEvent(FSEvent),
    FSMonitor(String),
    LuaError(String),
}
use anyhow::Result as AResult;
type Result = AResult<()>;

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
        screen: &mut Box<dyn UserInterface>,
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
                            if let TelnetEvents::DataSend(buffer) = parser.send_text(line.line()) {
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
            Event::Connect(connection) => {
                self.session.disconnect();
                spawn_connect_thread(self.session.clone(), connection);
                Ok(())
            }
            Event::Connected(id) => {
                let (writer, reader): (Sender<TelnetData>, Receiver<TelnetData>) = channel();
                spawn_receive_thread(self.session.clone());
                spawn_transmit_thread(self.session.clone(), reader);
                transmit_writer.replace(writer);
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
            Event::Disconnect => {
                if self.session.connected() {
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
                    screen.clear_tags()?;
                    screen.print_prompt(&Line::from(""));
                }
                Ok(())
            }
            Event::Reconnect => {
                let host = self.session.host();
                let port = self.session.port();
                let tls = self.session.tls();
                let verify = self.session.verify_cert();
                if !host.is_empty() && !port > 0 {
                    self.session
                        .main_writer
                        .send(Event::Connect(Connection::new(&host, port, tls, verify)))?;
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
            logger.log_str(&format!("{prefix}{line}"))?;
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

    pub fn handle_output_events(
        &self,
        event: Event,
        screen: &mut Box<dyn UserInterface>,
    ) -> Result {
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
            Event::SetPromptMask(mask) => {
                if let Ok(mut command_buffer) = self.session.command_buffer.lock() {
                    let mut lua_ctx = self.session.lua_script.lock().unwrap();
                    let updated_mask_table = command_buffer.set_mask(mask);
                    lua_ctx.set_prompt_mask_content(updated_mask_table);
                    let mut prompt_input = self.session.prompt_input.lock().unwrap();
                    *prompt_input = command_buffer.get_masked_buffer();
                    screen.print_prompt_input(&prompt_input, command_buffer.get_pos());
                }
                Ok(())
            }
            Event::ClearPromptMask => {
                if let Ok(mut command_buffer) = self.session.command_buffer.lock() {
                    command_buffer.clear_mask();
                    if let Ok(mut luascript) = self.session.lua_script.lock() {
                        luascript.set_prompt_mask_content(command_buffer.get_mask());
                    }
                    let mut prompt_input = self.session.prompt_input.lock().unwrap();
                    *prompt_input = command_buffer.get_masked_buffer();
                    screen.print_prompt_input(&prompt_input, command_buffer.get_pos());
                }
                Ok(())
            }
            Event::UserInputBuffer(input_buffer, pos) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    script.on_prompt_update(&input_buffer);
                }
                let mut prompt_input = self.session.prompt_input.lock().unwrap();
                *prompt_input = input_buffer;
                screen.print_prompt_input(&prompt_input, pos);
                Ok(())
            }
            Event::UserInputCursor(pos) => {
                let prompt_input = self.session.prompt_input.lock().unwrap();
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
            Event::ClearTags => {
                screen.clear_tags()?;
                Ok(())
            }
            Event::AddTag(tag) => screen.add_tag(&tag),
            _ => Err(BadEventRoutingError.into()),
        }
    }

    pub fn handle_scroll_events(
        &self,
        event: Event,
        screen: &mut Box<dyn UserInterface>,
    ) -> Result {
        match event {
            Event::ScrollLock(enabled) => {
                screen.scroll_lock(enabled)?;
                Ok(())
            }
            Event::ScrollUp => {
                screen.scroll_up()?;
                Ok(())
            }
            Event::ScrollDown => {
                screen.scroll_down()?;
                Ok(())
            }
            Event::ScrollTop => {
                screen.scroll_top()?;
                Ok(())
            }
            Event::ScrollBottom => {
                screen.reset_scroll()?;
                Ok(())
            }
            Event::FindForward(pattern) => {
                screen.find_down(&pattern)?;
                Ok(())
            }
            Event::FindBackward(pattern) => {
                screen.find_up(&pattern)?;
                Ok(())
            }
            _ => Err(BadEventRoutingError.into()),
        }
    }
}

pub(crate) fn spawn_quit_confirm_timeout_thread(
    writer: Sender<Event>,
    timeout: time::Duration,
) -> std::io::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("quit-confirm-timeout-thread".to_string())
        .spawn(move || {
            thread::sleep(timeout);
            writer.send(Event::QuitConfirmTimeout).unwrap();
        })
}

#[cfg(test)]
mod event_test {

    use std::sync::{Arc, Mutex};

    use mockall::predicate::eq;

    use crate::{model::Regex, session::SessionBuilder, timer::TimerEvent};

    use crate::io::MockLogWriter;
    use crate::ui::MockUserInterface;

    use super::*;

    fn build_session() -> (Session, Receiver<Event>, Receiver<TimerEvent>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let (timer_writer, timer_reader): (Sender<TimerEvent>, Receiver<TimerEvent>) = channel();
        let session = SessionBuilder::new()
            .main_writer(writer)
            .timer_writer(timer_writer)
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
    fn test_scrolling() {
        let (session, _reader, _timer_reader) = build_session();
        let mut screen = MockUserInterface::new();
        screen.expect_scroll_up().times(1).returning(|| Ok(()));
        screen.expect_scroll_top().times(1).returning(|| Ok(()));
        screen.expect_scroll_down().times(1).returning(|| Ok(()));
        screen.expect_reset_scroll().times(1).returning(|| Ok(()));
        screen
            .expect_scroll_lock()
            .times(1)
            .with(eq(true))
            .returning(|_| Ok(()));
        screen
            .expect_scroll_lock()
            .times(1)
            .with(eq(false))
            .returning(|_| Ok(()));
        let handler = EventHandler::from(&session);
        let mut screen: Box<dyn UserInterface> = Box::new(screen);
        assert!(handler
            .handle_scroll_events(Event::ScrollUp, &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollDown, &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollTop, &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollBottom, &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollLock(true), &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::ScrollLock(false), &mut screen)
            .is_ok());
    }

    #[test]
    fn test_find() {
        let (session, _reader, _timer_reader) = build_session();
        let re = Regex::new("test", None).unwrap();
        let mut screen = MockUserInterface::new();
        screen
            .expect_find_down()
            .times(1)
            .withf(|other| *other == Regex::new("test", None).unwrap())
            .returning(|_| Ok(()));
        screen
            .expect_find_up()
            .times(1)
            .withf(|other| *other == Regex::new("test", None).unwrap())
            .returning(|_| Ok(()));
        let handler = EventHandler::from(&session);
        let mut screen: Box<dyn UserInterface> = Box::new(screen);
        assert!(handler
            .handle_scroll_events(Event::FindBackward(re.clone()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_scroll_events(Event::FindForward(re), &mut screen)
            .is_ok());
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

        let line = Line::from("Output line");
        let mut screen: Box<dyn UserInterface> = Box::new(screen);
        assert!(handler
            .handle_output_events(Event::MudOutput(line.clone()), &mut screen)
            .is_ok());
        assert!(handler
            .handle_output_events(Event::Output(line), &mut screen)
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
    }

    #[test]
    fn test_spawn_quit_confirm_timeout_thread() {
        let (session, reader, _) = build_session();

        let handle = spawn_quit_confirm_timeout_thread(
            session.main_writer.clone(),
            time::Duration::from_millis(500),
        )
        .expect("unexpected err spawning quit confirm thread");

        handle
            .join()
            .expect("failed to join on quit confirm thread");
        let event = reader.recv().expect("failed to recv event");
        assert_eq!(event, Event::QuitConfirmTimeout);
    }
}
