use crate::{event::Event, net::TelnetHandler, session::Session};
use flate2::read::ZlibDecoder;
use log::{debug, error};
use std::{
    io::{Chain, Cursor, Read, Write},
    sync::mpsc::Receiver,
    thread,
};

use super::MudConnection;

type Decoder = ZlibDecoder<Chain<Cursor<Vec<u8>>, MudConnection>>;

pub const BUFFER_SIZE: usize = 32 * 1024;

struct MudReceiver {
    connection: MudConnection,
    decoder: Option<Decoder>,
}

impl MudReceiver {
    fn open_zlib_stream(&mut self, existing: Vec<u8>) {
        debug!("Opening Zlib stream");
        let chain = ZlibDecoder::new(Cursor::new(existing).chain(self.connection.clone()));
        self.decoder.replace(chain);
    }

    fn read_bytes(&mut self) -> Vec<u8> {
        let mut data = vec![0; BUFFER_SIZE];
        if let Some(decoder) = &mut self.decoder {
            debug!(
                "Waiting for zlib data... ({} ---> {})",
                decoder.total_in(),
                decoder.total_out()
            );
            match decoder.read(&mut data) {
                Ok(bytes_read) => {
                    debug!("Read {} bytes from zlib stream", bytes_read);
                    if bytes_read > 0 {
                        data = data[..bytes_read].to_vec();
                        debug!("Bytes: {:?}", data);
                    } else {
                        data = vec![];
                    }
                }
                Err(err) => {
                    error!("Error: {}", err.to_string());
                    data = vec![];
                }
            }
        } else if let Ok(bytes_read) = self.connection.read(&mut data) {
            debug!("Read {} bytes from stream", bytes_read);
            data = data[..bytes_read].to_vec();
            debug!("Bytes: {:?}", data);
        } else {
            data = vec![];
        }
        data
    }
}

impl From<&Session> for MudReceiver {
    fn from(session: &Session) -> Self {
        Self {
            connection: session.connection.lock().unwrap().clone(),
            decoder: None,
        }
    }
}

pub fn spawn_receive_thread(session: Session) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut mud_receiver = MudReceiver::from(&session);
        let writer = &session.main_writer;
        let mut telnet_handler = TelnetHandler::new(session.clone());
        let connection_id = session.connection_id();

        debug!("Receive stream spawned");
        let mut remaining_bytes = None;
        loop {
            if let Some(bytes) = remaining_bytes {
                mud_receiver.open_zlib_stream(bytes);
            }

            let bytes = mud_receiver.read_bytes();
            if bytes.is_empty() {
                writer
                    .send(Event::Info("Connection closed".to_string()))
                    .unwrap();
                writer.send(Event::Disconnect(connection_id)).unwrap();
                break;
            }

            remaining_bytes = telnet_handler.parse(&bytes);
        }
        debug!("Receive stream closing");
    })
}

pub fn spawn_transmit_thread(
    mut session: Session,
    transmit_read: Receiver<Option<Vec<u8>>>,
) -> thread::JoinHandle<()> {
    let connection = session.connection.lock().unwrap().clone();
    thread::spawn(move || {
        let mut connection = connection;
        let transmit_read = transmit_read;
        let connection_id = session.connection_id();
        debug!("Transmit stream spawned");
        while let Ok(Some(data)) = transmit_read.recv() {
            if let Err(info) = connection.write_all(data.as_slice()) {
                session.disconnect();
                let error = format!("Failed to write to socket: {}", info).to_string();
                session.send_event(Event::Error(error));
                session.send_event(Event::Disconnect(connection_id));
            }
        }
        debug!("Transmit stream closing");
    })
}
