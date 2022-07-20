use crate::event::Event;
use crate::net::OutputBuffer;
use crate::session::Session;
use libtelnet_rs::{
    events::TelnetEvents,
    telnet::{op_command as cmd, op_option as opt},
    Parser,
};
use log::debug;
use std::sync::{mpsc::Sender, Arc, Mutex};

#[derive(Default, Eq, PartialEq, Clone, Debug)]
pub enum TelnetMode {
    TerminatedPrompt,
    #[default]
    UnterminatedPrompt,
}

pub struct TelnetHandler {
    parser: Arc<Mutex<Parser>>,
    main_writer: Sender<Event>,
    output_buffer: Arc<Mutex<OutputBuffer>>,
    mode: TelnetMode,
}

impl TelnetHandler {
    pub fn new(session: Session) -> Self {
        Self {
            parser: session.telnet_parser,
            main_writer: session.main_writer,
            output_buffer: session.output_buffer,
            mode: TelnetMode::UnterminatedPrompt,
        }
    }
}

impl TelnetHandler {
    pub fn parse(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        let mut result = None;
        let events = if let Ok(mut parser) = self.parser.lock() {
            parser.receive(data)
        } else {
            vec![]
        };
        for event in events {
            match event {
                TelnetEvents::IAC(iac) => {
                    debug!("IAC: {}", iac.command);
                    match iac.command {
                        cmd::GA | cmd::EOR => {
                            if self.mode != TelnetMode::TerminatedPrompt {
                                debug!("Setting telnet mode: TerminatedPrompt");
                                if iac.command == cmd::GA {
                                    self.main_writer
                                        .send(Event::AddTag("GA".to_string()))
                                        .unwrap();
                                } else if iac.command == cmd::EOR {
                                    self.main_writer
                                        .send(Event::AddTag("EOR".to_string()))
                                        .unwrap();
                                }
                                self.mode = TelnetMode::TerminatedPrompt;
                                let mut output_buffer = self.output_buffer.lock().unwrap();
                                output_buffer.telnet_mode(&self.mode);
                            }
                            let mut buffer = self.output_buffer.lock().unwrap();
                            if buffer.has_new_data() {
                                let prompt = buffer.buffer_to_prompt(true);
                                debug!("IAC prompt: {}", prompt);
                                self.main_writer.send(Event::Prompt(prompt)).unwrap();
                            } else {
                                // Just flush
                                buffer.buffer_to_prompt(true);
                            }
                        }
                        _ => {}
                    }
                }
                TelnetEvents::Negotiation(neg) => {
                    debug!("Telnet negotiation: {} -> {}", neg.command, neg.option);
                    if let Ok(mut parser) = self.parser.lock() {
                        if neg.command == cmd::WILL || neg.command == cmd::DO {
                            parser._will(neg.option);
                            self.main_writer
                                .send(Event::ProtoEnabled(neg.option))
                                .unwrap();
                            if neg.option == opt::EOR {
                                self.main_writer
                                    .send(Event::AddTag("EOR".to_string()))
                                    .unwrap();
                            }
                        }
                    }
                }
                TelnetEvents::DecompressImmediate(buffer) => {
                    debug!("Breaking on buff: {:?}", &buffer);
                    result = Some(buffer.to_vec());
                    break;
                }
                TelnetEvents::Subnegotiation(data) => match data.option {
                    opt::MCCP2 => {
                        debug!("Initiated MCCP2 compression");
                        self.main_writer
                            .send(Event::AddTag("MCCP2".to_string()))
                            .unwrap();
                    }
                    opt => {
                        self.main_writer
                            .send(Event::ProtoSubnegRecv(opt, data.buffer))
                            .unwrap();
                    }
                },
                TelnetEvents::DataSend(msg) => {
                    debug!("Telnet sending: {:?}", msg);
                    if !msg.is_empty() {
                        self.main_writer.send(Event::ServerSend(msg)).unwrap();
                    }
                }
                TelnetEvents::DataReceive(msg) => {
                    debug!("Data receive: {:?}", msg);
                    if !msg.is_empty() && msg[0] != 0 {
                        if let Ok(mut output_buffer) = self.output_buffer.lock() {
                            let new_lines = output_buffer.receive(&msg);
                            for line in new_lines {
                                self.main_writer.send(Event::MudOutput(line)).unwrap();
                            }
                        };
                        self.handle_prompt();
                    }
                }
            };
        }
        result
    }

    pub fn handle_prompt(&mut self) {
        if self.mode == TelnetMode::UnterminatedPrompt {
            if let Ok(mut output_buffer) = self.output_buffer.lock() {
                if output_buffer.len() < 500 {
                    let prompt = output_buffer.buffer_to_prompt(false);
                    debug!("END prompt: {}", prompt);
                    self.main_writer.send(Event::Prompt(prompt)).unwrap();
                }
            }
        }
    }
}
