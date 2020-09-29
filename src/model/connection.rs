use crate::io::SaveData;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct Connection {
    pub host: String,
    pub port: u16,
    pub tls: Option<bool>,
}

impl Connection {
    pub fn new(host: &str, port: u16, tls: bool) -> Self {
        Self {
            host: host.to_owned(),
            port,
            tls: Some(tls),
        }
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tls = self.tls.unwrap_or_else(|| false);
        write!(f, "Host: {}, Port: {} TLS: {}", self.host, self.port, tls)
    }
}

pub type Servers = HashMap<String, Connection>;

impl SaveData for Servers {
    fn relative_path() -> PathBuf {
        PathBuf::from("data/servers.ron")
    }
}
