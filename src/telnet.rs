use crate::event::Event;
use crate::output_buffer::OutputBuffer;
use crate::session::{CommunicationOptions, Session};
use libtelnet_rs::{
    events::TelnetEvents,
    telnet::{op_command as cmd, op_option as opt},
    Parser,
};
use log::debug;
use std::sync::{mpsc::Sender, Arc, Mutex};

pub struct TelnetHandler {
    parser: Arc<Mutex<Parser>>,
    main_writer: Sender<Event>,
    output_buffer: Arc<Mutex<OutputBuffer>>,
    comops: Arc<Mutex<CommunicationOptions>>,
}

impl TelnetHandler {
    pub fn new(session: Session) -> Self {
        Self {
            parser: session.telnet_parser,
            main_writer: session.main_writer,
            output_buffer: session.output_buffer,
            comops: session.comops.clone(),
        }
    }
}

impl TelnetHandler {
    pub fn parse(&mut self, data: &[u8]) {
        if let Ok(mut parser) = self.parser.lock() {
            for event in parser.receive(data) {
                match event {
                    TelnetEvents::IAC(_) => {}
                    TelnetEvents::Negotiation(neg) => {
                        debug!("Telnet negotiation: {} -> {}", neg.command, neg.option);
                        if neg.option == opt::GMCP && neg.command == cmd::WILL {
                            parser._will(opt::GMCP);
                            self.main_writer
                                .send(Event::ProtoEnabled(opt::GMCP))
                                .unwrap();
                        }
                        if neg.option == opt::MCCP2 && neg.command == cmd::WILL {
                            parser._will(opt::MCCP2);
                            self.main_writer
                                .send(Event::ProtoEnabled(opt::MCCP2))
                                .unwrap();
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
                                if !output_buffer.is_empty() {
                                    output_buffer.buffer_to_prompt();
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
