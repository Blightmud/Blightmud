use libtelnet_rs::events::TelnetEvents;
use log::{debug, error, info};
use std::io::{Read, Write};
use std::sync::{
    atomic::Ordering,
    mpsc::{channel, Receiver, Sender},
};
use std::thread;

mod ansi;
mod command;
mod event;
mod output_buffer;
mod screen;
mod session;
mod telnet;

use crate::ansi::*;
use crate::command::spawn_input_thread;
use crate::event::Event;
use crate::screen::Screen;
use crate::session::{Session, SessionBuilder};
use crate::telnet::TelnetHandler;

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
                    session.send_event(Event::Error("Connection closed".to_string()));
                    session.send_event(Event::Disconnect);
                    break;
                }
            } else {
                session.send_event(Event::Error("Connection failed".to_string()));
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

fn start_logging() {
    simple_logging::log_to_file("logs/log.txt", log::LevelFilter::Debug).unwrap();
}

fn main() {
    start_logging();
    info!("Starting application");

    let (main_thread_write, main_thread_read): (Sender<Event>, Receiver<Event>) = channel();

    let mut session = SessionBuilder::new()
        .main_thread_writer(main_thread_write)
        .build();

    let _input_thread = spawn_input_thread(session.clone());

    {
        let mut screen = Screen::new();
        screen.setup();

        let mut transmit_writer: Option<Sender<TelnetData>> = None;
        let mut telnet_handler = TelnetHandler::new(session.clone());

        loop {
            if session.terminate.load(Ordering::Relaxed) {
                break;
            }
            if let Ok(event) = main_thread_read.recv() {
                match event {
                    Event::Prompt => {
                        let prompt_input = session.prompt_input.lock().unwrap();
                        let output_buffer = session.output_buffer.lock().unwrap();
                        screen.print_prompt(&output_buffer.prompt, &prompt_input);
                    }
                    Event::ServerSend(data) => {
                        if let Some(transmit_writer) = &transmit_writer {
                            transmit_writer.send(Some(data)).unwrap();
                        }
                    }
                    Event::ServerOutput(data) => {
                        telnet_handler.parse(&data);
                    }
                    Event::ServerInput(msg) => {
                        if session.connected.load(Ordering::Relaxed) {
                            if let Ok(mut parser) = session.telnet_parser.lock() {
                                if let TelnetEvents::DataSend(buffer) = parser.send_text(&msg) {
                                    if let Some(transmit_writer) = &transmit_writer {
                                        transmit_writer.send(Some(buffer)).unwrap();
                                    }
                                }
                            }
                        } else {
                            let msg = format!("{}[!!] No active session{}", FG_RED, DEFAULT);
                            session.main_thread_writer.send(Event::Output(msg)).unwrap();
                        }
                    }
                    Event::Output(msg) => {
                        screen.print_output(&msg);
                    }
                    Event::UserInputBuffer(input_buffer) => {
                        let mut prompt_input = session.prompt_input.lock().unwrap();
                        let output_buffer = session.output_buffer.lock().unwrap();
                        *prompt_input = input_buffer;
                        screen.print_prompt(&output_buffer.prompt, &prompt_input);
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
                        screen.print_output(&format!("{}[!!]{}{}", FG_RED, msg, DEFAULT));
                    }
                    Event::Info(msg) => {
                        screen.print_output(&format!("[**]{}", msg));
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
                        session.send_event(Event::UserInputBuffer(String::new()));
                    }
                    Event::Quit => {
                        session.terminate.store(true, Ordering::Relaxed);
                        session.disconnect();
                        break;
                    }
                };
                screen.flush();
            }
        }
        screen.reset();
    }

    debug!("Shutting down threads");
    session.close();
    //debug!("Joining threads");
    //input_thread.join().unwrap();
    info!("Shutting down");
}
