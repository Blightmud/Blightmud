use libtelnet_rs::{events::TelnetEvents, telnet::op_command as cmd, Parser};
use log::{debug, info};
use std::io::{stdin, stdout, Read, Write};
use std::sync::{
    atomic::Ordering,
    mpsc::{channel, Receiver, Sender},
};
use std::thread;
use termion::{event::Key, input::TermRead, raw::IntoRawMode, screen::AlternateScreen};

mod ansi;
mod output_buffer;
mod session;

use crate::ansi::*;
use crate::output_buffer::OutputBuffer;
use crate::session::{Session, SessionBuilder};

type TelnetData = Option<Vec<u8>>;

fn _spawn_receive_thread(session: Session) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        debug!("Receive stream spawned");
        let mut read_stream = session.stream.unwrap();
        let receive_write = session.output_writer;
        let ui_update = session.ui_update_notifier;

        loop {
            let mut data = vec![0; 1024];
            if let Ok(bytes_read) = read_stream.read(&mut data) {
                if bytes_read > 0 {
                    receive_write
                        .send(Some(Vec::from(data.split_at(bytes_read).0)))
                        .unwrap();
                    ui_update.send(true).unwrap();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        debug!("Receive stream closing");
    })
}

fn _spawn_transmit_thread(
    session: Session,
    transmit_read: Receiver<Option<Vec<u8>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        debug!("Transmit stream spawned");
        let transmit_read = transmit_read;
        let mut write_stream = session.stream.unwrap();
        while let Ok(Some(data)) = transmit_read.recv() {
            if let Err(info) = write_stream.write_all(data.as_slice()) {
                panic!("Failed to write to socket: {:?}", info);
            }
        }
        debug!("Transmit stream closing");
    })
}

fn spawn_input_thread(session: Session) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        debug!("Input stream spawned");
        let input_write = session.input_writer;
        let ui_update = session.ui_update_notifier;
        let input_buffer_write = session.input_buffer_write;
        let terminate = session.terminate;
        let stdin = stdin();
        let mut buffer = String::new();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('\n') => {
                    input_write.send(Some(buffer.clone())).unwrap();
                    buffer.clear();
                }
                Key::Char(c) => buffer.push(c),
                Key::Ctrl('c') => {
                    debug!("Caught ctrl-c, terminating");
                    terminate.store(true, Ordering::Relaxed);
                }
                Key::Backspace => {
                    buffer.pop();
                }
                _ => {}
            };
            input_buffer_write.send(buffer.clone()).unwrap();
            ui_update.send(true).unwrap();
            if terminate.load(Ordering::Relaxed) {
                break;
            }
        }
        debug!("Input stream closing");
    })
}

fn spawn_input_relay_thread(
    session: Session,
    input_read: Receiver<Option<String>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        debug!("Input relay stream spawned");
        let mut parser = Parser::new();
        let input_read = input_read;
        let transmit_writer = session.transmit_writer;
        let local_output_writer = session.local_output_writer;
        let ui_update = session.ui_update_notifier;
        let connected = session.connected.clone();

        while let Ok(Some(input)) = input_read.recv() {
            if connected.load(Ordering::Relaxed) {
                if let TelnetEvents::DataSend(data) = parser.send_text(input.as_str()) {
                    transmit_writer.send(Some(data)).unwrap();
                }
            } else {
                local_output_writer
                    .send(format!("{}[!!] No active session{}", FG_RED, DEFAULT).to_string())
                    .unwrap();
                ui_update.send(true).unwrap();
            }
        }
        debug!("Input relay stream closing");
    })
}

fn main() {
    simple_logging::log_to_file("logs/log.txt", log::LevelFilter::Debug).unwrap();
    info!("Starting application");

    let (receive_write, receive_read): (Sender<TelnetData>, Receiver<TelnetData>) = channel();
    let (transmit_write, _transmit_read): (Sender<TelnetData>, Receiver<TelnetData>) = channel();
    let (input_write, input_read): (Sender<Option<String>>, Receiver<Option<String>>) = channel();
    let (local_output_writer, local_output_reader): (Sender<String>, Receiver<String>) = channel();
    let (input_buffer_write, input_buffer_read): (Sender<String>, Receiver<String>) = channel();
    let (ui_update_input_write, ui_update_read): (Sender<bool>, Receiver<bool>) = channel();

    let mut session = SessionBuilder::new()
        .output_writer(receive_write)
        .local_output_writer(local_output_writer)
        .transmit_writer(transmit_write)
        .input_writer(input_write)
        .input_buffer_write(input_buffer_write)
        .ui_update_notifier(ui_update_input_write)
        .build();

    // Read socket stream
    //let receive_thread = spawn_receive_thread(session.clone());
    //let transmit_thread = spawn_transmit_thread(session.clone(), transmit_read);
    let input_thread = spawn_input_thread(session.clone());
    let relay_thread = spawn_input_relay_thread(session.clone(), input_read);

    {
        let (t_width, t_height) = termion::terminal_size().unwrap();
        let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        let output_line = t_height - 3;
        let prompt_line = t_height;
        let mut output_buffer = OutputBuffer::new();
        write!(screen, "{}{}", termion::clear::All, termion::cursor::Hide).unwrap();
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
        loop {
            if session.terminate.load(Ordering::Relaxed) {
                break;
            }
            if ui_update_read.recv().is_ok() {
                if let Ok(input_buffer) = input_buffer_read.try_recv() {
                    prompt_input = input_buffer.clone();
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
                if let Ok(msg) = local_output_reader.try_recv() {
                    write!(
                        screen,
                        "{}{}\r\n{}",
                        termion::cursor::Goto(1, output_line),
                        msg,
                        termion::cursor::Goto(1, prompt_line)
                    )
                    .unwrap();
                }
                if let Ok(Some(data)) = receive_read.try_recv() {
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
                                if !msg.is_empty() {
                                    session.transmit_writer.send(Some(msg)).unwrap();
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
                screen.flush().unwrap();
            }
        }
        writeln!(screen, "{}", ResetScrollRegion).unwrap(); // Reset scroll region
    }

    debug!("Shutting down threads");
    session.close();
    debug!("Joining threads");
    //receive_thread.join().unwrap();
    //transmit_thread.join().unwrap();
    input_thread.join().unwrap();
    relay_thread.join().unwrap();
    info!("Shutting down");
}
