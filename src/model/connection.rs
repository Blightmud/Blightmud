use crate::io::SaveData;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct Connection {
    pub host: String,
    pub port: u16,
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Host: {}, Port: {}", self.host, self.port)
    }
}

pub type Servers = HashMap<String, Connection>;

impl SaveData for Servers {
    fn relative_path() -> PathBuf {
        PathBuf::from("data/servers.ron")
    }
}
