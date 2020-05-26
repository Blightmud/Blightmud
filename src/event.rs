use crate::{
    io::SaveData,
    model::{Connection, Line, Servers},
    net::{spawn_receive_thread, spawn_transmit_thread},
    session::Session,
    ui::Screen,
    TelnetData,
};
use libtelnet_rs::events::TelnetEvents;
use log::debug;
use std::{
    error::Error,
    sync::{
        atomic::Ordering,
        mpsc::{channel, Receiver, Sender},
    },
};

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    Prompt,
    ServerSend(Vec<u8>),
    ServerInput(Line),
    MudOutput(Line),
    Output(Line),
    Error(String),
    Info(String),
    UserInputBuffer(String, usize),
    Connect(Connection),
    Connected,
    Disconnect(u16),
    Reconnect,
    AddServer(String, Connection),
    RemoveServer(String),
    LoadServer(String),
    ListServers,
    ProtoEnabled(u8),
    GMCPReceive(String),
    GMCPRegister(String),
    GMCPSend(String),
    AddTimedEvent(chrono::Duration, Option<u32>, u32),
    TimedEvent(u32),
    DropTimedEvent(u32),
    LoadScript(String),
    ResetScript,
    StartLogging(String, bool),
    StopLogging,
    ToggleSetting(String, String),
    ShowSetting(String),
    ScrollUp,
    ScrollDown,
    ScrollBottom,
    ShowHelp(String),
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

struct BadEventRoutingError;

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
        screen: &mut Screen,
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
            Event::ServerInput(line) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    if !script.check_for_alias_match(&line) {
                        screen.print_send(&line);
                        if let Ok(mut logger) = self.session.logger.lock() {
                            logger.log_line(&format!("> {}", &line))?;
                        }
                        if let Ok(mut parser) = self.session.telnet_parser.lock() {
                            if let TelnetEvents::DataSend(buffer) = parser.send_text(&line.line()) {
                                self.session.main_writer.send(Event::ServerSend(buffer))?;
                            }
                        }
                    } else {
                        script.get_output_lines().iter().for_each(|l| {
                            screen.print_output(l);
                        });
                    }
                }
                Ok(())
            }
            Event::Connect(Connection { host, port }) => {
                self.session.disconnect();
                if self.session.connect(&host, port) {
                    let (writer, reader): (Sender<TelnetData>, Receiver<TelnetData>) = channel();
                    spawn_receive_thread(self.session.clone());
                    spawn_transmit_thread(self.session.clone(), reader);
                    transmit_writer.replace(writer);
                } else {
                    screen.print_error(&format!("Failed to connect to {}:{}", host, port));
                }
                Ok(())
            }
            Event::Connected => {
                let host = self.session.host.lock().unwrap();
                let port = self.session.port.load(Ordering::Relaxed);
                debug!("Connected to {}:{}", host, port);
                screen.redraw_top_bar(&host, port)?;
                self.session
                    .lua_script
                    .lock()
                    .unwrap()
                    .on_connect(&host, port);
                Ok(())
            }
            Event::Disconnect(id) => {
                let disconnect = id == 0 || self.session.connection_id == id;
                if disconnect && self.session.connected.load(Ordering::Relaxed) {
                    self.session.disconnect();
                    screen.print_info(&format!(
                        "Disconnecting from: {}:{}",
                        self.session.host.lock().unwrap(),
                        self.session.port.load(Ordering::Relaxed)
                    ));
                    if let Some(transmit_writer) = &transmit_writer {
                        transmit_writer.send(None)?;
                    }
                    transmit_writer.take();
                    screen.redraw_top_bar("", 0)?;
                }
                Ok(())
            }
            Event::Reconnect => {
                let host = self.session.host.lock().unwrap().clone();
                let port = self.session.port.load(Ordering::Relaxed);
                if !host.is_empty() && !port > 0 {
                    self.session
                        .main_writer
                        .send(Event::Connect(Connection { host, port }))?;
                } else {
                    screen.print_error("Reconnect to what?");
                }
                Ok(())
            }
            _ => Err(BadEventRoutingError.into()),
        }
    }

    fn log_line(&self, line: &str) -> Result {
        if let Ok(mut logger) = self.session.logger.lock() {
            logger.log_line(line)?;
        }
        Ok(())
    }

    fn handle_logging(&self, event: Event) -> Result {
        match event {
            Event::MudOutput(line) | Event::Output(line) => self.log_line(&line.clean_line()),
            Event::Error(line) => self.log_line(&format!("[!!] {}", &line)),
            Event::Info(line) => self.log_line(&format!("[**] {}", &line)),
            Event::Prompt => {
                if let Ok(output_buffer) = self.session.output_buffer.lock() {
                    self.log_line(&output_buffer.prompt.clean_line())?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn handle_output_events(&self, event: Event, screen: &mut Screen) -> Result {
        self.handle_logging(event.clone())?;
        match event {
            Event::MudOutput(mut line) => {
                if let Ok(script) = self.session.lua_script.lock() {
                    script.check_for_trigger_match(&mut line);
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
            Event::Prompt => {
                let mut output_buffer = self.session.output_buffer.lock().unwrap();
                if let Ok(script) = self.session.lua_script.lock() {
                    script.check_for_prompt_trigger_match(&mut output_buffer.prompt);
                    script.get_output_lines().iter().for_each(|l| {
                        screen.print_output(l);
                    });
                }
                screen.print_prompt(&output_buffer.prompt);
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
            _ => Err(BadEventRoutingError.into()),
        }
    }

    pub fn handle_store_events(
        &mut self,
        event: Event,
        saved_servers: &mut Servers,
        screen: &mut Screen,
    ) -> Result {
        match event {
            Event::AddServer(name, connection) => {
                if saved_servers.contains_key(&name) {
                    self.session.main_writer.send(Event::Error(format!(
                        "Saved server already exists for {}",
                        name
                    )))?;
                    return Ok(());
                }

                saved_servers.insert(name.clone(), connection);
                saved_servers.save()?;

                self.session
                    .main_writer
                    .send(Event::Info(format!("Server added: {}", name)))?;

                Ok(())
            }
            Event::RemoveServer(name) => {
                if saved_servers.contains_key(&name) {
                    saved_servers.remove(&name);
                    saved_servers.save()?;

                    self.session
                        .main_writer
                        .send(Event::Info(format!("Server removed: {}", name)))?;
                } else {
                    self.session.main_writer.send(Event::Error(format!(
                        "Saved server does not exist: {}",
                        name
                    )))?;
                }

                Ok(())
            }
            Event::LoadServer(name) => {
                if saved_servers.contains_key(&name) {
                    let connection = saved_servers.get(&name).cloned().unwrap();
                    self.session.main_writer.send(Event::Connect(connection))?;
                } else {
                    screen.print_error(&format!("Saved server does not exist: {}", name));
                }

                Ok(())
            }
            Event::ListServers => {
                if saved_servers.is_empty() {
                    screen.print_info("There are no saved servers.");
                } else {
                    screen.print_info("Saved servers:");
                    screen.print_info("");
                    for server in saved_servers {
                        screen.print_info(&format!(" - Name: {}, {}", server.0, server.1));
                    }
                }

                Ok(())
            }
            _ => Err(BadEventRoutingError.into()),
        }
    }
}
