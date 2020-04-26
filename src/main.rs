use libtelnet_rs::{events::TelnetEvents, Parser, telnet::op_command as cmd};
use std::io::{Read, Write, stdin, stdout};
use std::net::TcpStream;
use std::sync::{
    mpsc::{Receiver, Sender, channel},
    atomic::{Ordering, AtomicBool},
    Arc,
};
use std::thread;
use std::time::Duration;
use termion::{
    screen::AlternateScreen,
    input::TermRead,
};

const HOST: &str = "achaea.com";
const PORT: u32 = 23;

fn main() {
    let server = format!("{}:{}", HOST, PORT);
    print!("Connecting to: {}...", server);
    let stream = TcpStream::connect(server).unwrap_or_else(|server| panic!("Failed to connect to {}", server));
    println!("Connected!");
    stream.set_read_timeout(Some(Duration::new(3, 0))).expect("Failed to set read timeout on socket");
    let read_stream = stream.try_clone().expect("Failed to create read_stream");
    let write_stream = stream.try_clone().expect("Failed to create write_stream");

    let terminate = Arc::new(AtomicBool::new(false));
    let ctrlc_terminate = terminate.clone();
    let terminate_receive = terminate.clone();
    let terminate_write = terminate.clone();

    ctrlc::set_handler(move || {
        println!("Caught ctrl-c");
        ctrlc_terminate.store(true, Ordering::Relaxed);
    }).unwrap();

    let (receive_write, receive_read): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();
    let (transmit_write, transmit_read): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();
    let (input_write, input_read): (Sender<String>, Receiver<String>) = channel();
    let input_transmit_write = transmit_write.clone();

    println!("Spawning receiver thread");
    thread::spawn(move || {
        let mut read_stream = read_stream;
        let receive_write = receive_write;
        let terminate = terminate_receive;
        loop {
            if terminate.load(Ordering::Relaxed) {
                read_stream.shutdown(std::net::Shutdown::Both).unwrap();
                break;
            }

            let mut data = vec![0;1024];
            if let Ok(bytes_read) = read_stream.read(&mut data) {
                if bytes_read > 0 {
                    receive_write.send(Vec::from(data.split_at(bytes_read).0)).unwrap();
                }
            }
        }
    });

    println!("Spawning transmitter thread");
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

    println!("Spawning input thread");
    thread::spawn(move || {
        let input_write = input_write;
        let stdin = stdin();
        loop {
            let mut stdin = stdin.lock();
            if let Ok(line) = stdin.read_line() {
                if let Some(line) = line {
                    input_write.send(line).unwrap();
                }
            }
        }
    });

    println!("Spawning input output thread");
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
        let mut screen = AlternateScreen::from(stdout());
        writeln!(screen, "{}{}", termion::clear::All, termion::cursor::Goto(1, 1)).unwrap();
        let mut parser = Parser::with_capacity(1024);
        'main_loop: loop {
            if terminate.load(Ordering::Relaxed) {
                writeln!(screen, "Exiting main thread").unwrap();
                break 'main_loop;
            }
            if let Ok(data) = receive_read.recv() {
                for event in parser.receive(data.as_slice()) {
                    match event {
                        TelnetEvents::IAC(iac) => {
                            if iac.command == cmd::GA {
                                writeln!(screen).unwrap();
                            }
                        },
                        TelnetEvents::Negotiation(_) => (),
                        TelnetEvents::Subnegotiation(_) => (),
                        TelnetEvents::DataSend(msg) => {
                            if !msg.is_empty() {
                                transmit_write.send(msg).unwrap();
                            }
                        },
                        TelnetEvents::DataReceive(msg) => {
                            if !msg.is_empty() {
                                write!(screen, "{}", std::str::from_utf8(msg.as_slice()).unwrap()).unwrap();
                            }
                        },
                    };
                }
            }
        }
    }

    stream.shutdown(std::net::Shutdown::Both).ok();
    println!("Shutting down");
}
