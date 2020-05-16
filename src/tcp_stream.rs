use crate::{
    event::Event,
    session::{CommunicationOptions, Session},
    telnet::TelnetHandler,
};
use flate2::read::ZlibDecoder;
use log::{debug, error};
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{mpsc::Receiver, Arc, Mutex},
    thread,
};

struct MudReceiver {
    reader: TcpStream,
    decoder: Option<ZlibDecoder<TcpStream>>,
    comops: Arc<Mutex<CommunicationOptions>>,
}

impl MudReceiver {
    fn check_open_zlib_stream(&mut self) {
        if self.decoder.is_none() {
            if let Ok(comops) = self.comops.lock() {
                if comops.mccp2 {
                    debug!("Opening Zlib stream");
                    self.decoder
                        .replace(ZlibDecoder::new(self.reader.try_clone().unwrap()));
                }
            }
        }
    }

    fn read_bytes(&mut self) -> Vec<u8> {
        let mut data = vec![0; 4096];
        if let Some(decoder) = &mut self.decoder {
            if let Ok(bytes_read) = decoder.read(&mut data) {
                data = data.split_at(bytes_read).0.to_vec();
            } else {
                data = vec![];
            }
        } else if let Ok(bytes_read) = self.reader.read(&mut data) {
                data = data.split_at(bytes_read).0.to_vec();
        } else {
            data = vec![];
        }
        data
    }
}

impl From<&Session> for MudReceiver {
    fn from(session: &Session) -> Self {
        Self {
            reader: session
                .stream
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .try_clone()
                .unwrap(),
            decoder: None,
            comops: session.comops.clone(),
        }
    }
}

pub fn spawn_receive_thread(session: Session) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut mud_receiver = MudReceiver::from(&session);
        let writer = &session.main_writer;
        let mut telnet_handler = TelnetHandler::new(session.clone());

        debug!("Receive stream spawned");
        loop {
            mud_receiver.check_open_zlib_stream();
            let bytes = mud_receiver.read_bytes();

            if bytes.is_empty() {
                writer
                    .send(Event::Info("Connection closed".to_string()))
                    .unwrap();
                writer.send(Event::Disconnect).unwrap();
                break;
            }

            telnet_handler.parse(&bytes);
        }
        debug!("Receive stream closing");
    })
}

pub fn spawn_transmit_thread(
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
