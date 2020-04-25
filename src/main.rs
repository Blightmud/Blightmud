use libtelnet_rs::{events::TelnetEvents, Parser, telnet::op_command as cmd};
use std::io::{Read, Write};
use std::net::TcpStream;

const HOST: &str = "aetolia.com";
const PORT: u32 = 23;

fn main() {
    let server = format!("{}:{}", HOST, PORT);
    let mut stream = TcpStream::connect(server).expect("Failed to connect to {}:{}");

    let mut parser = Parser::with_capacity(1024);
    loop {
        let mut data: Vec<u8> = vec![0; 1024];
        if let Ok(len) = stream.read(&mut data) {
            let events = parser.receive(data.split_at(len).0);
            for event in events {
                match event {
                    TelnetEvents::IAC(iac) => {
                        if iac.command == cmd::GA {
                            println!();
                        }
                    },
                    TelnetEvents::Negotiation(_) => (),
                    TelnetEvents::Subnegotiation(_) => (),
                    TelnetEvents::DataSend(msg) => {
                        if !msg.is_empty() {
                            stream.write_all(msg.as_slice()).unwrap();
                        }
                    },
                    TelnetEvents::DataReceive(msg) => {
                        if !msg.is_empty() {
                            print!("{}", std::str::from_utf8(msg.as_slice()).unwrap());
                        }
                    },
                };
            }
        }
    }
}
