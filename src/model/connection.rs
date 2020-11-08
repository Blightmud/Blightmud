use crate::io::SaveData;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct Connection {
    pub host: String,
    pub port: u16,
    pub tls: bool,
}

impl Connection {
    pub fn new(host: &str, port: u16, tls: bool) -> Self {
        Self {
            host: host.to_owned(),
            port,
            tls,
        }
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Host: {}, Port: {} TLS: {}",
            self.host, self.port, self.tls
        )
    }
}

pub type Servers = HashMap<String, Connection>;

impl SaveData for Servers {
    fn relative_path() -> PathBuf {
        crate::CONFIG_DIR.join("servers.ron")
    }

    fn is_pretty() -> bool {
        true
    }
}
