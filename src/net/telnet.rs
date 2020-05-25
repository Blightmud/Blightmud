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
    TerminatedPrompt,
    UnterminatedPrompt,
}

impl Default for TelnetMode {
    fn default() -> Self {
        TelnetMode::UnterminatedPrompt
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
            comops: session.comops.clone(),
            mode: TelnetMode::UnterminatedPrompt,
        }
    }
}

impl TelnetHandler {
    pub fn parse(&mut self, data: &[u8]) {
        if let Ok(mut parser) = self.parser.lock() {
            for event in parser.receive(data) {
                match event {
                    TelnetEvents::IAC(iac) => {
                        if iac.command == cmd::GA {
                            let mut buffer = self.output_buffer.lock().unwrap();
                            if self.mode == TelnetMode::UnterminatedPrompt {
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
                            Neg {
                                option: opt::TTYPE,
                                command: cmd::DO,
                            } => {
                                parser._will(opt::TTYPE);
                                self.main_writer
                                    .send(Event::ProtoEnabled(opt::TTYPE))
                                    .unwrap();
                            }
                            _ => {}
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
                        opt::TTYPE => {
                            if !data.buffer.is_empty() && data.buffer[0] == 1 {
                                debug!("TTYPE requested, responding");
                                if let Some(TelnetEvents::DataSend(data)) = parser.subnegotiation(
                                    opt::TTYPE,
                                    [&[cmd::IS][..], b"BLIGHTMUD"].concat(),
                                ) {
                                    self.main_writer.send(Event::ServerSend(data)).unwrap();
                                }
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
                                if !output_buffer.is_empty()
                                    && output_buffer.len() < 80
                                    && self.mode == TelnetMode::UnterminatedPrompt
                                {
                                    output_buffer.buffer_to_prompt(false);
                                    self.main_writer.send(Event::Prompt).unwrap();
                                }
                            }
                        }
                    }
                };
            }
        }
    }
}
