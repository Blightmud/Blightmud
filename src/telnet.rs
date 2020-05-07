use crate::event::Event;
use crate::output_buffer::OutputBuffer;
use crate::session::Session;
use libtelnet_rs::{
    events::TelnetEvents,
    telnet::{op_command as cmd, op_option as opt},
    Parser,
};
use log::debug;
use std::sync::{mpsc::Sender, Arc, Mutex};

pub struct TelnetHandler {
    parser: Arc<Mutex<Parser>>,
    main_thread_writer: Sender<Event>,
    output_buffer: Arc<Mutex<OutputBuffer>>,
}

impl TelnetHandler {
    pub fn new(session: Session) -> Self {
        Self {
            parser: session.telnet_parser,
            main_thread_writer: session.main_thread_writer,
            output_buffer: session.output_buffer,
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
                            if let Ok(mut output_buffer) = self.output_buffer.lock() {
                                output_buffer.buffer_to_prompt();
                                self.main_thread_writer.send(Event::Prompt).unwrap();
                            }
                        }
                    }
                    TelnetEvents::Negotiation(neg) => {
                        debug!("Telnet negotiation: {} -> {}", neg.command, neg.option);
                        if neg.option == opt::GMCP && neg.command == cmd::WILL {
                            if let Some(TelnetEvents::DataSend(data)) = parser._will(opt::GMCP) {
                                self.main_thread_writer
                                    .send(Event::ServerSend(data))
                                    .unwrap();
                                self.main_thread_writer
                                    .send(Event::ProtoEnabled(opt::GMCP))
                                    .unwrap();
                            }
                        }
                    }
                    TelnetEvents::Subnegotiation(data) => {
                        let msg = String::from_utf8_lossy(&data.buffer).to_mut().clone();
                        self.main_thread_writer
                            .send(Event::GMCPReceive(msg))
                            .unwrap();
                    }
                    TelnetEvents::DataSend(msg) => {
                        if !msg.is_empty() {
                            self.main_thread_writer
                                .send(Event::ServerSend(msg))
                                .unwrap();
                        }
                    }
                    TelnetEvents::DataReceive(msg) => {
                        if !msg.is_empty() {
                            if let Ok(mut output_buffer) = self.output_buffer.lock() {
                                let new_lines = output_buffer.receive(msg.as_slice());
                                for line in new_lines {
                                    self.main_thread_writer.send(Event::Output(line)).unwrap();
                                }
                            }
                        }
                    }
                };
            }
        }
    }
}
