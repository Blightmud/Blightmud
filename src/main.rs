use libtelnet_rs::{events::TelnetEvents, telnet::op_option as opt};
use log::{debug, error, info};
use signal_hook;
use std::io::{Read, Write};
use std::sync::{
    atomic::Ordering,
    mpsc::{channel, Receiver, Sender},
};
use std::thread;

mod ansi;
mod command;
mod event;
mod lua_script;
mod output_buffer;
mod screen;
mod session;
mod telnet;

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

fn register_terminal_resize_listener(session: Session) -> thread::JoinHandle<()> {
    let signals = signal_hook::iterator::Signals::new(&[signal_hook::SIGWINCH]).unwrap();
    let main_thread_writer = session.main_thread_writer;
    thread::spawn(move || {
        for _ in signals.forever() {
            main_thread_writer.send(Event::Redraw).unwrap();
        }
    })
}

fn start_logging() {
    simple_logging::log_to_file("logs/log.txt", log::LevelFilter::Debug).unwrap();
}

fn main() {
    start_logging();
    info!("Starting application");

    let (main_thread_write, main_thread_read): (Sender<Event>, Receiver<Event>) = channel();

    let session = SessionBuilder::new()
        .main_thread_writer(main_thread_write)
        .build();

    let _input_thread = spawn_input_thread(session.clone());
    let _signal_thread = register_terminal_resize_listener(session.clone());

    run(main_thread_read, session);

    info!("Shutting down");
}

fn run(main_thread_read: Receiver<Event>, mut session: Session) {
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
                    let output_buffer = session.output_buffer.lock().unwrap();
                    if let Ok(script) = session.lua_script.lock() {
                        script.check_for_prompt_trigger_match(&output_buffer.prompt);
                    }
                    screen.print_prompt(&output_buffer.prompt);
                }
                Event::ServerSend(data) => {
                    if let Some(transmit_writer) = &transmit_writer {
                        transmit_writer.send(Some(data)).unwrap();
                    }
                }
                Event::ServerOutput(data) => {
                    telnet_handler.parse(&data);
                }
                Event::ServerInput(msg, check_alias) => {
                    if let Ok(script) = session.lua_script.lock() {
                        if !check_alias || !script.check_for_alias_match(&msg) {
                            screen.print_send(&msg);
                            if session.connected.load(Ordering::Relaxed) {
                                if let Ok(mut parser) = session.telnet_parser.lock() {
                                    if let TelnetEvents::DataSend(buffer) = parser.send_text(&msg) {
                                        if let Some(transmit_writer) = &transmit_writer {
                                            transmit_writer.send(Some(buffer)).unwrap();
                                        }
                                    }
                                }
                            } else {
                                session
                                    .main_thread_writer
                                    .send(Event::Error("No active session".to_string()))
                                    .unwrap();
                            }
                        }
                    }
                }
                Event::MudOutput(msg) => {
                    if let Ok(script) = session.lua_script.lock() {
                        if script.check_for_trigger_match(&msg) {
                            screen.print_output("Trigger match");
                        }
                    }
                    screen.print_output(&msg);
                }
                Event::Output(msg) => {
                    screen.print_output(&msg);
                }
                Event::UserInputBuffer(input_buffer) => {
                    let mut prompt_input = session.prompt_input.lock().unwrap();
                    *prompt_input = input_buffer;
                    screen.print_prompt_input(&prompt_input);
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
                Event::Connected => {
                    debug!("Connected to {}:{}", session.host, session.port);
                }
                Event::ProtoEnabled(proto) => {
                    if let opt::GMCP = proto {
                        let mut parser = session.telnet_parser.lock().unwrap();
                        if let Some(event) = parser.subnegotiation_text(
                            opt::GMCP,
                            "Core.Hello {\"Client\":\"rs-mud\",\"Version\":\"0.1.0\"}",
                        ) {
                            if let TelnetEvents::DataSend(data) = event {
                                debug!("Sending GMCP Core.Hello");
                                session
                                    .main_thread_writer
                                    .send(Event::ServerSend(data))
                                    .unwrap();
                            }
                        } else {
                            error!("Failed to send GMCP Core.Hello");
                        }
                    }
                }
                Event::GMCPReceive(_) => {
                    //screen.print_output(&format!("[GMCP]: {}", msg));
                }
                Event::ScrollUp => screen.scroll_up(),
                Event::ScrollDown => screen.scroll_down(),
                Event::ScrollBottom => screen.reset_scroll(),
                Event::Error(msg) => {
                    screen.print_error(&msg);
                }
                Event::Info(msg) => {
                    screen.print_info(&msg);
                }
                Event::LoadScript(path) => {
                    info!("Loading script: {}", path);
                    let mut lua = session.lua_script.lock().unwrap();
                    if let Err(err) = lua.load_script(&path) {
                        screen.print_error(&format!("Failed to load file: {}", err));
                    }
                }
                Event::Redraw => {
                    screen.setup();
                    screen.reset_scroll();
                }
                Event::Disconnect => {
                    session.disconnect();
                    screen.print_info(&format!(
                        "Disconnecting from: {}:{}",
                        session.host, session.port
                    ));
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
    session.close();
}
