use crate::event::Event;
use crate::net::OutputBuffer;
use crate::session::{CommunicationOptions, Session};
use libtelnet_rs::{
    events::{TelnetEvents, TelnetNegotiation as Neg},
    telnet::{op_command as cmd, op_option as opt},
    Parser,
};
use log::debug;
use std::sync::{mpsc::Sender, Arc, Mutex};

#[derive(Eq, PartialEq)]
enum TelnetMode {
    Undefined,
    TerminatedPrompt,
    UnterminatedPrompt,
    NoPrompt,
}

impl Default for TelnetMode {
    fn default() -> Self {
        TelnetMode::Undefined
    }
}

pub struct TelnetHandler {
    parser: Arc<Mutex<Parser>>,
    main_writer: Sender<Event>,
    output_buffer: Arc<Mutex<OutputBuffer>>,
    comops: Arc<Mutex<CommunicationOptions>>,
    mode: TelnetMode,
}

impl TelnetHandler {
    pub fn new(session: Session) -> Self {
        Self {
            parser: session.telnet_parser,
            main_writer: session.main_writer,
            output_buffer: session.output_buffer,
            comops: session.comops,
            mode: TelnetMode::Undefined,
        }
    }
}

impl TelnetHandler {
    pub fn parse(&mut self, data: &[u8]) {
        let events = if let Ok(mut parser) = self.parser.lock() {
            parser.receive(data)
        } else {
            vec![]
        };
        for event in events {
            match event {
                TelnetEvents::IAC(iac) => {
                    if iac.command == cmd::GA {
                        let mut buffer = self.output_buffer.lock().unwrap();
                        if self.mode != TelnetMode::TerminatedPrompt {
                            debug!("Setting telnet mode: TerminatedPrompt");
                            self.mode = TelnetMode::TerminatedPrompt;
                            buffer.flush();
                        } else {
                            buffer.buffer_to_prompt(true);
                            self.main_writer.send(Event::Prompt).unwrap();
                        }
                    }
                }
                TelnetEvents::Negotiation(neg) => {
                    debug!("Telnet negotiation: {} -> {}", neg.command, neg.option);
                    if let Ok(mut parser) = self.parser.lock() {
                        match neg {
                            Neg {
                                option: opt::GMCP,
                                command: cmd::WILL,
                            } => {
                                parser._will(opt::GMCP);
                                self.main_writer
                                    .send(Event::ProtoEnabled(opt::GMCP))
                                    .unwrap();
                            }
                            Neg {
                                option: opt::MCCP2,
                                command: cmd::WILL,
                            } => {
                                parser._will(opt::MCCP2);
                                self.main_writer
                                    .send(Event::ProtoEnabled(opt::MCCP2))
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }
                TelnetEvents::Subnegotiation(data) => match data.option {
                    opt::GMCP => {
                        let msg = String::from_utf8_lossy(&data.buffer).to_mut().clone();
                        self.main_writer.send(Event::GMCPReceive(msg)).unwrap();
                    }
                    opt::MCCP2 => {
                        debug!("Initiated MCCP2 compression");
                        if let Ok(mut comops) = self.comops.lock() {
                            comops.mccp2 = true;
                        }
                    }
                    _ => {}
                },
                TelnetEvents::DataSend(msg) => {
                    debug!("Telnet sending: {:?}", msg);
                    if !msg.is_empty() {
                        self.main_writer.send(Event::ServerSend(msg)).unwrap();
                    }
                }
                TelnetEvents::DataReceive(msg) => {
                    if !msg.is_empty() {
                        if let Ok(mut output_buffer) = self.output_buffer.lock() {
                            let new_lines = output_buffer.receive(msg.as_slice());
                            for line in new_lines {
                                self.main_writer.send(Event::MudOutput(line)).unwrap();
                            }
                        };
                        self.handle_prompt();
                    }
                }
            };
        }
    }

    pub fn handle_prompt(&mut self) {
        if let Ok(mut output_buffer) = self.output_buffer.lock() {
            if self.mode == TelnetMode::Undefined {
                if output_buffer.is_empty() {
                    debug!("Setting telnet mode: NoPrompt");
                    self.mode = TelnetMode::NoPrompt;
                } else if !output_buffer.is_empty() && output_buffer.len() < 80 {
                    debug!("Setting telnet mode: UnterminatedPrompt");
                    self.mode = TelnetMode::UnterminatedPrompt;
                }
            }

            if self.mode == TelnetMode::UnterminatedPrompt
                && !output_buffer.is_empty()
                && output_buffer.len() < 80
            {
                output_buffer.buffer_to_prompt(false);
                self.main_writer.send(Event::Prompt).unwrap();
            }
        }
    }
}
