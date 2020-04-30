use libtelnet_rs::{events::TelnetEvents, telnet::op_command as cmd, Parser};
use log::{debug, error, info};
use std::io::{stdout, Read, Write};
use std::sync::{
    atomic::Ordering,
    mpsc::{channel, Receiver, Sender},
};
use std::thread;
use termion::{raw::IntoRawMode, screen::AlternateScreen};

mod ansi;
mod command;
mod event;
mod output_buffer;
mod session;

use crate::ansi::*;
use crate::command::spawn_input_thread;
use crate::event::Event;
use crate::output_buffer::OutputBuffer;
use crate::session::{Session, SessionBuilder};

type TelnetData = Option<Vec<u8>>;

fn spawn_receive_thread(mut session: Session) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut read_stream = if let Ok(stream) = &session.stream.lock() {
            stream.as_ref().unwrap().try_clone().unwrap()
        } else {
            error!("Failed to spawn receive stream without a live connection");
            panic!("Failed to spawn receive stream");
        };
        let writer = &session.main_thread_writer;

        debug!("Receive stream spawned");
        loop {
            let mut data = vec![0; 1024];
            if let Ok(bytes_read) = read_stream.read(&mut data) {
                if bytes_read > 0 {
                    writer
                        .send(Event::ServerOutput(Vec::from(data.split_at(bytes_read).0)))
                        .unwrap();
                } else {
                    session.send_event(Event::Error("Zero bytes received from socket".to_string()));
                    session.send_event(Event::Disconnect);
                    break;
                }
            } else {
                session.send_event(Event::Error("Failed to read from socket".to_string()));
                session.send_event(Event::Disconnect);
                break;
            }
        }
        debug!("Receive stream closing");
    })
}

fn spawn_transmit_thread(
    mut session: Session,
    transmit_read: Receiver<Option<Vec<u8>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut write_stream = if let Ok(stream) = &session.stream.lock() {
            stream.as_ref().unwrap().try_clone().unwrap()
        } else {
            error!("Failed to spawn transmit stream without a live connection");
            panic!("Failed to spawn transmit stream");
        };
        let transmit_read = transmit_read;
        debug!("Transmit stream spawned");
        while let Ok(Some(data)) = transmit_read.recv() {
            if let Err(info) = write_stream.write_all(data.as_slice()) {
                session.disconnect();
                let error = format!("Failed to write to socket: {}", info).to_string();
                session.send_event(Event::Error(error));
                session.send_event(Event::Disconnect);
            }
        }
        debug!("Transmit stream closing");
    })
}

fn main() {
    simple_logging::log_to_file("logs/log.txt", log::LevelFilter::Debug).unwrap();
    info!("Starting application");

    let (main_thread_writer, main_thread_read): (Sender<Event>, Receiver<Event>) = channel();

    let mut session = SessionBuilder::new()
        .main_thread_writer(main_thread_writer)
        .build();

    let _input_thread = spawn_input_thread(session.clone());

    {
        let (t_width, t_height) = termion::terminal_size().unwrap();
        let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        let output_line = t_height - 3;
        let prompt_line = t_height;
        let mut output_buffer = OutputBuffer::new();
        write!(screen, "{}{}", termion::clear::All, termion::cursor::Show).unwrap();
        write!(
            screen,
            "{}{}",
            ScrollRegion(1, output_line),
            DisableOriginMode
        )
        .unwrap(); // Set scroll region, non origin mode
        write!(screen, "{}", termion::cursor::Goto(1, output_line + 1)).unwrap();
        write!(screen, "{:_<1$}", "", t_width as usize).unwrap(); // Print separator
        screen.flush().unwrap();
        let mut parser = Parser::with_capacity(1024);
        let mut prompt_input = String::new();
        let mut transmit_writer: Option<Sender<TelnetData>> = None;

        loop {
            if session.terminate.load(Ordering::Relaxed) {
                break;
            }
            if let Ok(event) = main_thread_read.recv() {
                match event {
                    Event::ServerOutput(data) => {
                        for event in parser.receive(data.as_slice()) {
                            match event {
                                TelnetEvents::IAC(iac) => {
                                    if iac.command == cmd::GA {
                                        output_buffer.buffer_to_prompt();
                                        write!(
                                            screen,
                                            "{}{}{}{}",
                                            termion::cursor::Goto(1, prompt_line),
                                            termion::clear::AfterCursor,
                                            output_buffer.prompt,
                                            prompt_input,
                                        )
                                        .unwrap();
                                    }
                                }
                                TelnetEvents::Negotiation(_) => (),
                                TelnetEvents::Subnegotiation(_) => (),
                                TelnetEvents::DataSend(msg) => {
                                    if let Some(transmit_writer) = &transmit_writer {
                                        if !msg.is_empty() {
                                            transmit_writer.send(Some(msg)).unwrap();
                                        }
                                    }
                                }
                                TelnetEvents::DataReceive(msg) => {
                                    if !msg.is_empty() {
                                        let new_lines = output_buffer.receive(msg.as_slice());
                                        write!(screen, "{}", termion::cursor::Goto(1, output_line))
                                            .unwrap();
                                        for line in new_lines {
                                            write!(screen, "{}\r\n", line.trim_end(),).unwrap();
                                        }
                                        write!(screen, "{}", termion::cursor::Goto(1, prompt_line))
                                            .unwrap();
                                    }
                                }
                            };
                        }
                    }
                    Event::ServerInput(msg) => {
                        if session.connected.load(Ordering::Relaxed) {
                            if let TelnetEvents::DataSend(buffer) = parser.send_text(&msg) {
                                if let Some(transmit_writer) = &transmit_writer {
                                    transmit_writer.send(Some(buffer)).unwrap();
                                }
                            }
                        } else {
                            let msg = format!("{}[!!] No active session{}", FG_RED, DEFAULT);
                            session
                                .main_thread_writer
                                .send(Event::LocalOutput(msg))
                                .unwrap();
                        }
                    }
                    Event::LocalOutput(msg) => {
                        write!(
                            screen,
                            "{}{}\r\n{}",
                            termion::cursor::Goto(1, output_line),
                            msg,
                            termion::cursor::Goto(1, prompt_line)
                        )
                        .unwrap();
                    }
                    Event::UserInputBuffer(input_buffer) => {
                        prompt_input = input_buffer;
                        write!(
                            screen,
                            "{}{}{} {}",
                            termion::cursor::Goto(1, prompt_line),
                            termion::clear::AfterCursor,
                            output_buffer.prompt,
                            prompt_input,
                        )
                        .unwrap();
                    }
                    Event::Connect(host, port) => {
                        session.disconnect();
                        if session.connect(&host, port) {
                            let (writer, reader): (Sender<TelnetData>, Receiver<TelnetData>) =
                                channel();
                            spawn_receive_thread(session.clone());
                            spawn_transmit_thread(session.clone(), reader);
                            transmit_writer.replace(writer);
                        } else {
                            session
                                .main_thread_writer
                                .send(Event::Error(
                                    format!("Failed to connect to {}:{}", host, port).to_string(),
                                ))
                                .unwrap();
                        }
                    }
                    Event::Error(msg) => {
                        write!(
                            screen,
                            "{}{}[!!] {}{}\r\n{}",
                            termion::cursor::Goto(1, output_line),
                            FG_RED,
                            msg,
                            DEFAULT,
                            termion::cursor::Goto(1, prompt_line)
                        )
                        .unwrap();
                    }
                    Event::Info(msg) => {
                        write!(
                            screen,
                            "{}[**] {}\r\n{}",
                            termion::cursor::Goto(1, output_line),
                            msg,
                            termion::cursor::Goto(1, prompt_line),
                        )
                        .unwrap();
                    }
                    Event::LoadScript(_) => {}
                    Event::Disconnect => {
                        session.disconnect();
                        let msg = format!("Disconnecting from: {}:{}", session.host, session.port)
                            .to_string();
                        session.send_event(Event::Info(msg));
                        if let Some(transmit_writer) = &transmit_writer {
                            transmit_writer.send(None).unwrap();
                        }
                        transmit_writer = None;
                        output_buffer.prompt.clear();
                        session.send_event(Event::UserInputBuffer(String::new()));
                    }
                    Event::Quit => {
                        session.terminate.store(true, Ordering::Relaxed);
                        session.disconnect();
                        break;
                    }
                };
                screen.flush().unwrap();
            }
        }
        writeln!(screen, "{}", ResetScrollRegion).unwrap(); // Reset scroll region
    }

    debug!("Shutting down threads");
    session.close();
    //debug!("Joining threads");
    //input_thread.join().unwrap();
    info!("Shutting down");
}
