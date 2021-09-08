use std::{
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use anyhow::{bail, Result};

pub fn open_tcp_stream(host: &str, port: u16) -> Result<TcpStream> {
    let mut addr_iter = (host, port).to_socket_addrs()?;

    let timeout = Duration::new(3, 0);
    let stream = addr_iter.find_map(|addr| TcpStream::connect_timeout(&addr, timeout).ok());

    if let Some(stream) = stream {
        Ok(stream)
    } else {
        bail!("Invalid connection params")
    }
}
