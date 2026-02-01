use crate::event::Event;
use crate::net::telnet::TelnetHandler;
use crate::session::Session;
use flate2::read::ZlibDecoder;
use libmudtelnet::bytes::Bytes;
use log::{debug, error};
use mio::net::TcpStream as MioTcpStream;
use mio::{Events, Interest, Poll, Token, Waker};
use rustls::ClientConnection;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

const TCP_TOKEN: Token = Token(0);
const WAKER_TOKEN: Token = Token(1);

const READ_BUFFER_SIZE: usize = 32 * 1024;
const POLL_TIMEOUT: Duration = Duration::from_millis(10);

/// Connection state that can be either plain TCP or TLS
enum ConnectionState {
    Plain(MioTcpStream),
    Tls {
        stream: MioTcpStream,
        tls: ClientConnection,
    },
}

impl ConnectionState {
    fn stream_mut(&mut self) -> &mut MioTcpStream {
        match self {
            ConnectionState::Plain(s) => s,
            ConnectionState::Tls { stream, .. } => stream,
        }
    }
}

/// A buffer that implements Read, allowing bytes to be pushed and read.
/// Returns WouldBlock when empty instead of EOF, so the ZlibDecoder
/// knows to wait for more data rather than treating it as end of stream.
struct StreamBuffer {
    inner: Rc<RefCell<VecDeque<u8>>>,
}

impl StreamBuffer {
    fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(VecDeque::new())),
        }
    }

    fn push(&self, data: &[u8]) {
        self.inner.borrow_mut().extend(data);
    }
}

impl Clone for StreamBuffer {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl Read for StreamBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut inner = self.inner.borrow_mut();
        if inner.is_empty() {
            return Err(io::Error::new(ErrorKind::WouldBlock, "no data available"));
        }

        let len = buf.len().min(inner.len());
        for (i, byte) in inner.drain(..len).enumerate() {
            buf[i] = byte;
        }
        Ok(len)
    }
}

/// Zlib decompression state for MCCP2
struct ZlibState {
    /// The stream buffer for feeding compressed data to the decoder
    buffer: StreamBuffer,
    /// The zlib decoder - created once and persists for the connection
    decoder: Option<ZlibDecoder<StreamBuffer>>,
}

impl ZlibState {
    fn new() -> Self {
        Self {
            buffer: StreamBuffer::new(),
            decoder: None,
        }
    }

    fn start_decompression_with(&mut self, initial_data: Vec<u8>) -> io::Result<Vec<u8>> {
        debug!(
            "Starting zlib decompression with {} initial bytes",
            initial_data.len()
        );
        // Create decoder once with a clone of the buffer (shares the same underlying VecDeque)
        self.decoder = Some(ZlibDecoder::new(self.buffer.clone()));

        // Decompress the initial data and return it
        self.decompress(&initial_data)
    }

    fn is_active(&self) -> bool {
        self.decoder.is_some()
    }

    fn decompress(&mut self, data: &[u8]) -> io::Result<Vec<u8>> {
        if let Some(decoder) = &mut self.decoder {
            // Push new compressed data to the shared buffer
            self.buffer.push(data);

            // Read all available decompressed data
            let mut output = Vec::new();
            let mut buf = [0u8; READ_BUFFER_SIZE];
            loop {
                match decoder.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => output.extend_from_slice(&buf[..n]),
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) => return Err(e),
                }
            }

            Ok(output)
        } else {
            // Not in decompression mode, return data as-is
            Ok(data.to_vec())
        }
    }
}

/// The main network event loop that handles both reading and writing
/// in a single thread using mio for non-blocking I/O.
pub struct NetworkEventLoop {
    poll: Poll,
    _waker: Arc<Waker>,
    connection: ConnectionState,
    write_buffer: Vec<u8>,
    transmit_receiver: Receiver<Option<Bytes>>,
    main_writer: Sender<Event>,
    telnet_handler: TelnetHandler,
    zlib_state: ZlibState,
    shutdown: bool,
}

impl NetworkEventLoop {
    /// Create a new event loop for a plain TCP connection
    pub fn new_plain(
        stream: TcpStream,
        transmit_receiver: Receiver<Option<Bytes>>,
        session: Session,
    ) -> io::Result<Self> {
        let poll = Poll::new()?;
        let waker = Arc::new(Waker::new(poll.registry(), WAKER_TOKEN)?);

        // Convert std TcpStream to mio TcpStream
        stream.set_nonblocking(true)?;
        let mut mio_stream = MioTcpStream::from_std(stream);

        // Register for readable events initially
        poll.registry()
            .register(&mut mio_stream, TCP_TOKEN, Interest::READABLE)?;

        let main_writer = session.main_writer.clone();
        let telnet_handler = TelnetHandler::new(session);

        Ok(Self {
            poll,
            _waker: waker,
            connection: ConnectionState::Plain(mio_stream),
            write_buffer: Vec::new(),
            transmit_receiver,
            main_writer,
            telnet_handler,
            zlib_state: ZlibState::new(),
            shutdown: false,
        })
    }

    /// Create a new event loop for a TLS connection
    pub fn new_tls(
        stream: TcpStream,
        tls: ClientConnection,
        transmit_receiver: Receiver<Option<Bytes>>,
        session: Session,
    ) -> io::Result<Self> {
        let poll = Poll::new()?;
        let waker = Arc::new(Waker::new(poll.registry(), WAKER_TOKEN)?);

        // Convert std TcpStream to mio TcpStream
        stream.set_nonblocking(true)?;
        let mut mio_stream = MioTcpStream::from_std(stream);

        // Register for readable events initially
        poll.registry().register(
            &mut mio_stream,
            TCP_TOKEN,
            Interest::READABLE | Interest::WRITABLE,
        )?;

        let main_writer = session.main_writer.clone();
        let telnet_handler = TelnetHandler::new(session);

        Ok(Self {
            poll,
            _waker: waker,
            connection: ConnectionState::Tls {
                stream: mio_stream,
                tls,
            },
            write_buffer: Vec::new(),
            transmit_receiver,
            main_writer,
            telnet_handler,
            zlib_state: ZlibState::new(),
            shutdown: false,
        })
    }

    /// Run the event loop until shutdown
    pub fn run(&mut self) {
        debug!("Network event loop starting");
        let mut events = Events::with_capacity(128);

        while !self.shutdown {
            // Check for outgoing data from the transmit channel
            self.check_transmit_channel();

            if self.shutdown {
                debug!("Shutdown requested via transmit channel");
                break;
            }

            // Update interest based on current state
            if let Err(e) = self.update_interest() {
                error!("Failed to update interest: {}", e);
                break;
            }

            // Poll for events
            match self.poll.poll(&mut events, Some(POLL_TIMEOUT)) {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => {
                    error!("Poll error: {}", e);
                    break;
                }
            }

            for event in events.iter() {
                match event.token() {
                    TCP_TOKEN => {
                        if event.is_readable() {
                            if let Err(e) = self.do_read() {
                                if e.kind() != ErrorKind::WouldBlock {
                                    debug!("Read error, closing connection: {}", e);
                                    self.shutdown = true;
                                    break;
                                }
                            }
                        }

                        if event.is_writable() {
                            if let Err(e) = self.do_write() {
                                if e.kind() != ErrorKind::WouldBlock {
                                    error!("Write error: {}", e);
                                    self.shutdown = true;
                                    break;
                                }
                            }
                        }
                    }
                    WAKER_TOKEN => {
                        debug!("Waker event");
                    }
                    _ => {}
                }
            }
        }

        debug!("Network event loop shutting down");
        let _ = self
            .main_writer
            .send(Event::Info("Connection closed".to_string()));
        let _ = self.main_writer.send(Event::Disconnect);
    }

    /// Check the transmit channel for outgoing data
    fn check_transmit_channel(&mut self) {
        loop {
            match self.transmit_receiver.try_recv() {
                Ok(Some(data)) => {
                    debug!("Queuing {} bytes for transmission", data.len());
                    self.write_buffer.extend_from_slice(&data);
                }
                Ok(None) => {
                    // None signals shutdown
                    debug!("Received shutdown signal");
                    self.shutdown = true;
                    return;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    debug!("Transmit channel disconnected");
                    self.shutdown = true;
                    return;
                }
            }
        }
    }

    /// Update mio interest registration based on current state
    fn update_interest(&mut self) -> io::Result<()> {
        let mut interest = Interest::READABLE;

        match &self.connection {
            ConnectionState::Plain(_) => {
                if !self.write_buffer.is_empty() {
                    interest = interest.add(Interest::WRITABLE);
                }
            }
            ConnectionState::Tls { tls, .. } => {
                // For TLS, we need WRITABLE if:
                // 1. We have application data to send
                // 2. TLS layer wants to write (handshake, alerts, etc.)
                if !self.write_buffer.is_empty() || tls.wants_write() {
                    interest = interest.add(Interest::WRITABLE);
                }
            }
        }

        self.poll
            .registry()
            .reregister(self.connection.stream_mut(), TCP_TOKEN, interest)
    }

    /// Perform non-blocking read
    fn do_read(&mut self) -> io::Result<()> {
        let mut read_buf = [0u8; READ_BUFFER_SIZE];
        let mut received_chunks: Vec<Vec<u8>> = Vec::new();

        // First, read all available data into chunks
        let read_result = match &mut self.connection {
            ConnectionState::Plain(stream) => {
                loop {
                    match stream.read(&mut read_buf) {
                        Ok(0) => {
                            // EOF - connection closed
                            break Err(io::Error::new(ErrorKind::ConnectionReset, "EOF"));
                        }
                        Ok(n) => {
                            received_chunks.push(read_buf[..n].to_vec());
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            break Ok(());
                        }
                        Err(e) => break Err(e),
                    }
                }
            }
            ConnectionState::Tls { stream, tls } => {
                // Read TLS records from the socket
                'outer: loop {
                    match tls.read_tls(stream) {
                        Ok(0) => {
                            // EOF - connection closed
                            break Err(io::Error::new(ErrorKind::ConnectionReset, "EOF"));
                        }
                        Ok(_) => {
                            // Process the TLS records
                            let state = match tls.process_new_packets() {
                                Ok(state) => state,
                                Err(e) => {
                                    error!("TLS error: {}", e);
                                    break Err(io::Error::new(ErrorKind::InvalidData, e));
                                }
                            };

                            // Check for peer closure
                            if state.peer_has_closed() {
                                break Err(io::Error::new(
                                    ErrorKind::ConnectionReset,
                                    "TLS peer closed",
                                ));
                            }

                            // Read decrypted data
                            loop {
                                match tls.reader().read(&mut read_buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        received_chunks.push(read_buf[..n].to_vec());
                                    }
                                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                                    Err(e) => break 'outer Err(e),
                                }
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            break Ok(());
                        }
                        Err(e) => break Err(e),
                    }
                }
            }
        };

        // Now process all received chunks (borrow of self.connection is released)
        for chunk in received_chunks {
            self.handle_received_data(&chunk)?;
        }

        read_result
    }

    /// Perform non-blocking write
    fn do_write(&mut self) -> io::Result<()> {
        match &mut self.connection {
            ConnectionState::Plain(stream) => {
                while !self.write_buffer.is_empty() {
                    match stream.write(&self.write_buffer) {
                        Ok(0) => {
                            return Err(io::Error::new(ErrorKind::WriteZero, "write zero"));
                        }
                        Ok(n) => {
                            self.write_buffer.drain(..n);
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            ConnectionState::Tls { stream, tls } => {
                // First, write any pending application data to TLS
                while !self.write_buffer.is_empty() {
                    match tls.writer().write(&self.write_buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            self.write_buffer.drain(..n);
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e),
                    }
                }

                // Then flush TLS records to the socket
                while tls.wants_write() {
                    match tls.write_tls(stream) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle received data by passing it through telnet handler
    fn handle_received_data(&mut self, data: &[u8]) -> io::Result<()> {
        debug!("Received {} bytes", data.len());

        // Handle zlib decompression if active
        let data = if self.zlib_state.is_active() {
            match self.zlib_state.decompress(data) {
                Ok(decompressed) => decompressed,
                Err(e) => {
                    error!("Zlib decompression error: {}", e);
                    return Err(e);
                }
            }
        } else {
            data.to_vec()
        };

        if data.is_empty() {
            return Ok(());
        }

        // Parse through telnet handler
        // The telnet handler returns Some(remaining_bytes) when MCCP2 starts
        if let Some(remaining) = self.telnet_handler.parse(&data) {
            // Start zlib decompression and decompress the remaining data
            if let Ok(decompressed) = self.zlib_state.start_decompression_with(remaining) {
                if !decompressed.is_empty() {
                    self.telnet_handler.parse(&decompressed);
                }
            }
        }

        Ok(())
    }
}
