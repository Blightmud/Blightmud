use crate::{
    event::Event,
    net::TelnetHandler,
    session::{CommunicationOptions, Session},
};
use flate2::read::ZlibDecoder;
use libtelnet_rs::telnet::op_option as opt;
use log::{debug, error};
use std::{
    io::{Chain, Cursor, Read, Write},
    sync::{mpsc::Receiver, Arc, Mutex},
    thread,
};

use super::MudConnection;

type Decoder = ZlibDecoder<Chain<Cursor<Vec<u8>>, MudConnection>>;

pub const BUFFER_SIZE: usize = 32 * 1024;

struct MudReceiver {
    connection: MudConnection,
    decoder: Option<Decoder>,
    comops: Arc<Mutex<CommunicationOptions>>,
    last_chunk: Vec<u8>,
}

impl MudReceiver {
    // If there were compressed bytes in the last chunk they are extracted here
    fn extract_compressed_bytes_from_last(&self) -> Option<&[u8]> {
        if let Some(item) = self
            .last_chunk
            .iter()
            .enumerate()
            .rfind(|item| *item.1 == opt::MCCP2)
        {
            // ... MCCP2 IAC SE, hence the + 3
            Some(&self.last_chunk[item.0 + 3..])
        } else {
            None
        }
    }

    fn check_open_zlib_stream(&mut self) {
        if self.decoder.is_none() {
            if let Ok(comops) = self.comops.lock() {
                if comops.mccp2 {
                    debug!("Opening Zlib stream");
                    let existing =
                        if let Some(existing_bytes) = self.extract_compressed_bytes_from_last() {
                            existing_bytes.to_vec()
                        } else {
                            vec![]
                        };

                    let chain = Cursor::new(existing).chain(self.connection.clone());
                    let decoder = ZlibDecoder::new(chain);
                    self.decoder.replace(decoder);
                }
            }
        }
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
        } else {
            data = vec![];
        }
        self.last_chunk = data.clone();
        data
    }
}

impl From<&Session> for MudReceiver {
    fn from(session: &Session) -> Self {
        Self {
            connection: session.connection.lock().unwrap().clone(),
            decoder: None,
            comops: session.comops.clone(),
            last_chunk: vec![],
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
        loop {
            mud_receiver.check_open_zlib_stream();
            let bytes = mud_receiver.read_bytes();

            if bytes.is_empty() {
                writer
                    .send(Event::Info("Connection closed".to_string()))
                    .unwrap();
                writer.send(Event::Disconnect(connection_id)).unwrap();
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
