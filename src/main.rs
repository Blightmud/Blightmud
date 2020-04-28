use libtelnet_rs::{events::TelnetEvents, telnet::op_command as cmd, Parser};
use std::io::{stdin, stdout, Read, Write};
use std::net::TcpStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, Receiver, Sender},
    Arc,
};
use std::thread;
use std::time::Duration;
use termion::{event::Key, input::TermRead, raw::IntoRawMode, screen::AlternateScreen};

mod output_buffer;
use crate::output_buffer::OutputBuffer;

const HOST: &str = "achaea.com";
const PORT: u32 = 23;

fn main() {
    let server = format!("{}:{}", HOST, PORT);
    print!("Connecting to: {}...", server);
    let stream = TcpStream::connect(server)
        .unwrap_or_else(|server| panic!("Failed to connect to {}", server));
    println!("Connected!");
    stream
        .set_read_timeout(Some(Duration::new(3, 0)))
        .expect("Failed to set read timeout on socket");
    let read_stream = stream.try_clone().expect("Failed to create read_stream");
    let write_stream = stream.try_clone().expect("Failed to create write_stream");

    let terminate = Arc::new(AtomicBool::new(false));
    let terminate_receive = terminate.clone();
    let terminate_write = terminate.clone();
    let terminate_input = terminate.clone();

    let (receive_write, receive_read): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();
    let (transmit_write, transmit_read): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();
    let (input_write, input_read): (Sender<String>, Receiver<String>) = channel();
    let (input_buffer_write, input_buffer_read): (Sender<String>, Receiver<String>) = channel();
    let input_transmit_write = transmit_write.clone();
    let (ui_update_input_write, ui_update_read): (Sender<bool>, Receiver<bool>) = channel();
    let ui_update_receive_write = ui_update_input_write.clone();

    thread::spawn(move || {
        let mut read_stream = read_stream;
        let receive_write = receive_write;
        let terminate = terminate_receive;
        let ui_update = ui_update_receive_write;
        loop {
            if terminate.load(Ordering::Relaxed) {
                read_stream.shutdown(std::net::Shutdown::Both).unwrap();
                break;
            }

            let mut data = vec![0; 1024];
            if let Ok(bytes_read) = read_stream.read(&mut data) {
                if bytes_read > 0 {
                    receive_write
                        .send(Vec::from(data.split_at(bytes_read).0))
                        .unwrap();
                    ui_update.send(true).unwrap();
                }
            }
        }
    });

    thread::spawn(move || {
        let transmit_read = transmit_read;
        let terminate = terminate_write;
        let mut write_stream = write_stream;
        'transmit_loop: loop {
            if terminate.load(Ordering::Relaxed) {
                write_stream.shutdown(std::net::Shutdown::Both).unwrap();
                break 'transmit_loop;
            }

            if let Ok(data) = transmit_read.recv() {
                if let Err(info) = write_stream.write_all(data.as_slice()) {
                    panic!("Failed to write to socket: {:?}", info);
                }
            }
        }
    });

    thread::spawn(move || {
        let input_write = input_write;
        let ui_update = ui_update_input_write;
        let input_buffer_write = input_buffer_write;
        let terminate = terminate_input;
        let stdin = stdin();
        let mut buffer = String::new();

        for c in stdin.keys() {
            match c.unwrap() {
                Key::Char('\n') => {
                    input_write.send(buffer.clone()).unwrap();
                    buffer.clear();
                }
                Key::Char(c) => buffer.push(c),
                Key::Ctrl('c') => terminate.store(true, Ordering::Relaxed),
                Key::Backspace => {
                    buffer.pop();
                }
                _ => {}
            };
            input_buffer_write.send(buffer.clone()).unwrap();
            ui_update.send(true).unwrap();
        }
    });

    thread::spawn(move || {
        let mut parser = Parser::new();
        let input_read = input_read;
        let input_transmit_write = input_transmit_write;

        loop {
            if let Ok(input) = input_read.recv() {
                if let TelnetEvents::DataSend(data) = parser.send_text(input.as_str()) {
                    input_transmit_write.send(data).unwrap();
                }
            }
        }
    });

    {
        let (t_width, t_height) = termion::terminal_size().unwrap();
        let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        let output_line = t_height - 3;
        let prompt_line = t_height;
        let mut output_buffer = OutputBuffer::new();
        write!(screen, "{}{}", termion::clear::All, termion::cursor::Hide).unwrap();
        write!(screen, "\x1b[1;{}r\x1b[?6l", output_line).unwrap(); // Set scroll region, non origin mode
        write!(screen, "{}", termion::cursor::Goto(1, output_line + 1)).unwrap();
        write!(screen, "{:_<1$}", "", t_width as usize).unwrap(); // Print separator
        let mut parser = Parser::with_capacity(1024);
        let mut prompt_input = String::new();
        'main_loop: loop {
            if terminate.load(Ordering::Relaxed) {
                writeln!(screen, "Exiting main thread").unwrap();
                break 'main_loop;
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
                if let Ok(data) = receive_read.try_recv() {
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
                                    transmit_write.send(msg).unwrap();
                                }
                            }
                            TelnetEvents::DataReceive(msg) => {
                                if !msg.is_empty() {
                                    let new_lines = output_buffer.receive(msg.as_slice());
                                    write!(screen, "{}", termion::cursor::Goto(1, output_line))
                                        .unwrap();
                                    for line in new_lines {
                                        write!(screen, "> {}\r\n", line.trim_end(),).unwrap();
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
        writeln!(screen, "\x1b[r").unwrap(); // Reset scroll region
    }

    stream.shutdown(std::net::Shutdown::Both).ok();
}
