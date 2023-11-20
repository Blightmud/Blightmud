use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use anyhow::{anyhow, Result};
use log::debug;
use socket2::{Socket, TcpKeepalive};

use crate::io::SaveData;
use crate::model::{self, KEEPALIVE_ENABLED};

/// Connect to a remote host and port, returning a `TcpStream` if successful.
///
/// This function will resolve potential IP addresses for the given host/port combination and
/// prefer connecting to an IPv6 address if available, falling back to IPv4 if necessary.
///
/// Unless disabled by setting the` KEEPALIVE_ENABLED` setting to false the streams returned
/// by this function will have [TCP keepalive](https://en.wikipedia.org/wiki/Keepalive#TCP_keepalive)
/// configured. This is important for Telnet connections since the protocol itself has no
/// application layer keepalive.
pub fn open_tcp_stream(host: &str, port: u16) -> Result<TcpStream> {
    let keepalive = model::Settings::try_load()?
        .get(KEEPALIVE_ENABLED)
        .unwrap_or(true);
    stream_with_options(host, port, keepalive)
}

fn stream_with_options(host: &str, port: u16, keepalive: bool) -> Result<TcpStream> {
    let sock = Socket::from(opportunistic_connect(prepare_addresses(host, port)?)?);
    if keepalive {
        debug!("enabling TCP keepalive");
        // Values are loosely based on Mudlet's settings, but tuned to be a little more aggressive.
        // E.g. a shorter wait before sending keepalives, a shorter wait between keepalives, and
        // fewer retries before giving up.
        // https://github.com/Mudlet/Mudlet/blob/31ea3079e63735a344379e714117e4f1ad6b2f1b/src/ctelnet.cpp#L3052-L3138
        sock.set_tcp_keepalive(
            &TcpKeepalive::new()
                // How long will the connection be allowed to sit idle before the first keepalive
                // packet is sent?
                .with_time(Duration::from_secs(30))
                // How long should we wait between sending keepalive packets?
                .with_interval(Duration::from_secs(5))
                // How many keepalive packets should we send before deciding a connection is dead?
                .with_retries(5),
        )?;
    }
    Ok(sock.into())
}

// Attempt to connect to each IP address for a remote host and port, returning a `TcpStream` if
// successful and continuing to try addresses sequentially until one succeeds or we run out.
fn opportunistic_connect(addrs: Vec<SocketAddr>) -> Result<TcpStream> {
    let mut last_error = anyhow!("no addresses resolved");
    for addr in addrs {
        debug!("attempting to connect to {}", addr);
        match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(stream) => {
                debug!("connected to {addr}");
                return Ok(stream);
            }
            Err(err) => {
                debug!("failed to connect to {addr}: {err}");
                last_error = err.into()
            }
        }
    }
    Err(last_error)
}

// Lookup IP addresses for the given host/port, returning a Vec that interleaves IPv6/IPv4 addresses.
// This makes it easy to prefer IPv6 addresses, but fall-back to IPv4 as we attempt connecting to
// each address in sequence.
fn prepare_addresses(host: &str, port: u16) -> Result<Vec<SocketAddr>> {
    debug!("resolving IP addresses for {host}:{port}");
    let (addrs_v4, addrs_v6): (Vec<_>, Vec<_>) =
        (host, port).to_socket_addrs()?.partition(|a| match a {
            SocketAddr::V4(_) => true,
            SocketAddr::V6(_) => false,
        });
    let mut addrs = Vec::with_capacity(addrs_v4.len() + addrs_v6.len());
    let (mut left, mut right) = (addrs_v6.into_iter(), addrs_v4.into_iter());
    while let Some(a) = left.next() {
        addrs.push(a);
        std::mem::swap(&mut left, &mut right);
    }
    addrs.extend(right);
    debug!("resolved {} potential addresses", addrs.len());
    Ok(addrs)
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread;

    use socket2::Socket;

    use crate::net::util::stream_with_options;

    #[test]
    fn test_keepalive_disable() {
        // Start a dummy TCP server on a random port.
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx): (Sender<bool>, Receiver<bool>) = channel();
        let server_handle = thread::spawn(move || {
            for stream in listener.incoming() {
                let _stream = stream.unwrap();
                if rx.recv().unwrap() {
                    break;
                }
            }
        });

        // Creating a stream with keepalive=true should result in a socket configured
        // with keepalive enabled.
        let stream =
            stream_with_options(addr.ip().to_string().as_str(), addr.port(), true).unwrap();
        let sock = Socket::from(stream);
        assert!(sock.keepalive().unwrap());
        drop(sock);

        // And creating a stream with keepalive=false should result in a socket configured
        // without keepalive enabled.
        let stream =
            stream_with_options(addr.ip().to_string().as_str(), addr.port(), false).unwrap();
        let sock = Socket::from(stream);
        assert!(!sock.keepalive().unwrap());

        // Shut down the server.
        tx.send(true).unwrap();
        server_handle.join().unwrap();
    }
}
