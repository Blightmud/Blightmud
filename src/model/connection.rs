use crate::io::SaveData;

use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub struct Connection {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub verify_cert: bool,
}

impl Connection {
    pub fn new(host: &str, port: u16, tls: bool, verify_cert: bool) -> Self {
        Self {
            host: host.to_owned(),
            port,
            tls,
            verify_cert,
        }
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Host: {}, Port: {} TLS: {} Verify: {}",
            self.host, self.port, self.tls, self.verify_cert
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

#[cfg(test)]
mod test_connection {

    use super::*;

    #[test]
    fn confirm_disp() {
        let conn = Connection::new("host.com", 8080, true, true);
        assert_eq!(
            format!("{}", conn),
            "Host: host.com, Port: 8080 TLS: true Verify: true".to_string()
        );
        let conn = Connection::new("host.com", 4000, false, false);
        assert_eq!(
            format!("{}", conn),
            "Host: host.com, Port: 4000 TLS: false Verify: false".to_string()
        );
    }
}
