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
    #[serde(skip)] // Don't persist - derived from server key
    pub name: Option<String>,
}

impl Connection {
    pub fn new(host: &str, port: u16, tls: bool, verify_cert: bool) -> Self {
        Self {
            host: host.to_owned(),
            port,
            tls,
            verify_cert,
            name: None,
        }
    }

    pub fn with_name(
        host: &str,
        port: u16,
        tls: bool,
        verify_cert: bool,
        name: Option<String>,
    ) -> Self {
        Self {
            host: host.to_owned(),
            port,
            tls,
            verify_cert,
            name,
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

    #[test]
    fn test_new_creates_connection_without_name() {
        let conn = Connection::new("example.com", 23, false, false);
        assert_eq!(conn.host, "example.com");
        assert_eq!(conn.port, 23);
        assert!(!conn.tls);
        assert!(!conn.verify_cert);
        assert!(conn.name.is_none());
    }

    #[test]
    fn test_with_name_creates_connection_with_name() {
        let conn =
            Connection::with_name("example.com", 23, true, true, Some("my_server".to_string()));
        assert_eq!(conn.host, "example.com");
        assert_eq!(conn.port, 23);
        assert!(conn.tls);
        assert!(conn.verify_cert);
        assert_eq!(conn.name, Some("my_server".to_string()));
    }

    #[test]
    fn test_with_name_no_name() {
        let conn = Connection::with_name("test.com", 4000, false, true, None);
        assert_eq!(conn.host, "test.com");
        assert_eq!(conn.port, 4000);
        assert!(!conn.tls);
        assert!(conn.verify_cert);
        assert!(conn.name.is_none());
    }

    #[test]
    fn test_connection_clone() {
        let conn = Connection::new("clone.com", 5555, true, false);
        let cloned = conn.clone();
        assert_eq!(conn, cloned);
    }

    #[test]
    fn test_connection_equality() {
        let conn1 = Connection::new("equal.com", 1234, true, true);
        let conn2 = Connection::new("equal.com", 1234, true, true);
        assert_eq!(conn1, conn2);
    }

    #[test]
    fn test_connection_inequality() {
        let conn1 = Connection::new("one.com", 1234, true, true);
        let conn2 = Connection::new("two.com", 1234, true, true);
        assert_ne!(conn1, conn2);

        let conn3 = Connection::new("one.com", 5678, true, true);
        assert_ne!(conn1, conn3);
    }

    #[test]
    fn test_connection_debug() {
        let conn = Connection::new("debug.com", 9999, false, true);
        let debug_str = format!("{:?}", conn);
        assert!(debug_str.contains("debug.com"));
        assert!(debug_str.contains("9999"));
    }
}
