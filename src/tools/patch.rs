use {
    crate::{
        event::Event,
        fs,
        io::SaveData,
        model::{Connection, Servers, Settings},
        tts::TTSSettings,
    },
    lazy_static::lazy_static,
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, path::PathBuf, sync::mpsc::Sender},
};

#[cfg(not(test))]
use crate::DATA_DIR;

#[cfg(not(test))]
lazy_static! {
    pub static ref V2_SERVERS_PATH: PathBuf = DATA_DIR.join("data/servers.ron");
    pub static ref V2_SETTINGS_PATH: PathBuf = DATA_DIR.join("config/settings.ron");
    pub static ref V2_TTS_SETTINGS_PATH: PathBuf = DATA_DIR.join("data/tts_settings.ron");
}

/// Migrates settings.ron and servers.ron from DATA_DIR to CONFIG_DIR.
/// This happened in the v3.0.0 release.
/// See https://github.com/LiquidityC/Blightmud/pull/202
pub fn migrate_v2_settings_and_servers(main_writer: Sender<Event>) {
    macro_rules! send_error {
        ($msg:expr) => { main_writer.send(Event::Error($msg.into())).unwrap(); };
        ($fmt:literal, $($args:expr,)+) => { send_error!(format!($fmt, $($args,)+)) };
        ($fmt:literal, $($args:expr),+) => { send_error!(format!($fmt, $($args,)+)) };
    }

    macro_rules! send_info {
        ($msg:expr) => { main_writer.send(Event::Info($msg.into())).unwrap(); };
        ($fmt:literal, $($args:expr,)+) => { send_info!(format!($fmt, $($args,)+)) };
        ($fmt:literal, $($args:expr),+) => { send_info!(format!($fmt, $($args,)+)) };
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    struct V2Settings {
        settings: HashMap<String, bool>,
    }

    impl SaveData for V2Settings {
        fn relative_path() -> PathBuf {
            V2_SETTINGS_PATH.clone()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    struct V2Connection {
        host: String,
        port: u16,
        tls: Option<bool>,
    }

    impl From<V2Connection> for Connection {
        fn from(v2: V2Connection) -> Connection {
            Connection {
                host: v2.host,
                port: v2.port,
                tls: v2.tls.unwrap_or_default(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    #[serde(transparent)]
    struct V2Servers {
        servers: HashMap<String, V2Connection>,
    }

    impl SaveData for V2Servers {
        fn relative_path() -> PathBuf {
            V2_SERVERS_PATH.clone()
        }
    }

    impl From<V2Servers> for Servers {
        fn from(v2: V2Servers) -> Servers {
            let mut servers = HashMap::new();
            for (k, v) in v2.servers {
                servers.insert(k, v.into());
            }
            servers
        }
    }

    if V2_SERVERS_PATH.exists() {
        if Servers::relative_path().exists() {
            send_error!("Can't migrate servers.ron:");
            send_error!(
                "servers.ron already exists at {:?}",
                Servers::relative_path()
            );
        } else {
            let old_servers = V2Servers::load();
            let new_servers: Servers = old_servers.into();
            new_servers.save();
            let _ = fs::remove_file(&*V2_SERVERS_PATH);
            send_info!(
                "Migrated servers.ron from {:?} to {:?}",
                *V2_SERVERS_PATH,
                Servers::relative_path()
            );
        }
    }

    if V2_SETTINGS_PATH.exists() {
        if Settings::relative_path().exists() {
            send_error!("Can't migrate settings.ron:");
            send_error!(
                "settings.ron already exists at {:?}",
                Settings::relative_path()
            );
        } else {
            let old_settings = V2Settings::load();
            let new_settings = Settings::from(old_settings.settings);
            new_settings.save();
            let _ = fs::remove_file(&*V2_SETTINGS_PATH);
            send_info!(
                "Migrated settings.ron from {:?} to {:?}",
                *V2_SETTINGS_PATH,
                Settings::relative_path()
            );
        }
    }

    if V2_TTS_SETTINGS_PATH.exists() {
        if TTSSettings::relative_path().exists() {
            send_error!(
                "tts_settings.ron already exists at {:?}",
                TTSSettings::relative_path()
            );
        } else {
            fs::rename(&*V2_TTS_SETTINGS_PATH, &*TTSSettings::relative_path()).unwrap();
            send_info!(
                "Migrated tts_settings.ron from {:?} to {:?}",
                *V2_TTS_SETTINGS_PATH,
                TTSSettings::relative_path(),
            );
        }
    }
}
