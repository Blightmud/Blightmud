use {
    crate::{
        event::Event,
        fs,
        io::SaveData,
        model::{Connection, Servers, Settings},
        tts::TTSSettings,
        DATA_DIR,
    },
    lazy_static::lazy_static,
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, path::PathBuf, sync::mpsc::Sender},
};

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
                verify_cert: false,
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
            TTSSettings::relative_path()
                .parent()
                .map(|dir| fs::create_dir_all(&dir));
            fs::rename(&*V2_TTS_SETTINGS_PATH, &TTSSettings::relative_path()).unwrap();
            send_info!(
                "Migrated tts_settings.ron from {:?} to {:?}",
                *V2_TTS_SETTINGS_PATH,
                TTSSettings::relative_path(),
            );
        }
    }
}

#[cfg(test)]
lazy_static! {
    pub static ref V2_SERVERS_PATH: PathBuf = DATA_DIR.join("v2_servers.ron");
    pub static ref V2_SETTINGS_PATH: PathBuf = DATA_DIR.join("v2_settings.ron");
    pub static ref V2_TTS_SETTINGS_PATH: PathBuf = DATA_DIR.join("v2_tts_settings.ron");
}

#[cfg(test)]
#[allow(dead_code)]
mod patch_test {
    use {
        super::*,
        crate::model::*,
        std::{io::Write, panic, sync::mpsc},
    };

    fn setup() {
        let paths = [
            TTSSettings::relative_path(),
            Settings::relative_path(),
            Servers::relative_path(),
            DATA_DIR.join("test"),
        ];
        for path in paths.iter() {
            path.parent().map(|dir| fs::create_dir_all(&dir));
        }
    }

    fn cleanup() {
        let _ = fs::remove_dir_all(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".run/test"));
    }

    fn create_v2_settings_file() {
        let v2_settings = r#"(settings:{"tts_enabled":true,"logging_enabled":true,"save_history":false,"confirm_quit":false,"mouse_enabled":true})"#;
        let mut file = fs::File::create(&*V2_SETTINGS_PATH).unwrap();
        file.write_all(v2_settings.as_bytes()).unwrap();
    }

    fn create_v2_tts_settings_file() {
        let v2_tts_settings = r#"(echo_keys:true,rate:5)"#;
        let mut file = fs::File::create(&*V2_TTS_SETTINGS_PATH).unwrap();
        file.write_all(v2_tts_settings.as_bytes()).unwrap();
    }

    fn create_v2_servers_file() {
        let v2_servers = r#"{"lotr":(host:"lotr-mud.com",port:5555,tls:Some(true)),"starwars":(host:"starwars.mud.com",port:1212,tls:Some(false)),"mymud2":(host:"mud2",port:1234,tls:Some(false)),"mymud":(host:"mud",port:6789,tls:Some(false))}"#;
        let mut file = fs::File::create(&*V2_SERVERS_PATH).unwrap();
        file.write_all(v2_servers.as_bytes()).unwrap();
    }

    macro_rules! assert_event {
        ($event:expr, $event_type:pat) => {{
            if !matches!($event, $event_type) {
                println!("{:?}", $event);
            }
            assert!(matches!($event, $event_type));
        }};
    }

    #[test]
    fn test_v2_migrator() {
        let orig_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            cleanup();
            orig_hook(info);
        }));

        let tests = [
            test_migrate_v2_servers,
            test_migrate_v2_settings,
            test_migrate_v2_tts_settings,
            test_doesnt_migrate_over_existing_settings_file,
            test_doesnt_migrate_over_existing_tts_settings_file,
            test_doesnt_migrate_over_existing_servers_file,
        ];

        for test in tests.iter() {
            setup();
            test();
            cleanup();
        }
    }

    fn test_migrate_v2_settings() {
        create_v2_settings_file();

        let (tx, rx) = mpsc::channel();
        migrate_v2_settings_and_servers(tx);

        let info = rx.recv().unwrap();
        assert_event!(info, Event::Info(..));

        let settings = Settings::try_load().unwrap();
        assert!(settings.get(LOGGING_ENABLED).unwrap());
        assert!(settings.get(TTS_ENABLED).unwrap());
        assert!(settings.get(MOUSE_ENABLED).unwrap());
        assert!(!settings.get(SAVE_HISTORY).unwrap());
        assert!(!settings.get(CONFIRM_QUIT).unwrap());
    }

    fn test_migrate_v2_tts_settings() {
        create_v2_tts_settings_file();

        let (tx, rx) = mpsc::channel();
        migrate_v2_settings_and_servers(tx);

        let info = rx.recv().unwrap();
        assert_event!(info, Event::Info(..));

        let settings = TTSSettings::try_load().unwrap();
        assert!(settings.echo_keys);
        assert_eq!(5.0, settings.rate);
    }

    fn test_migrate_v2_servers() {
        create_v2_servers_file();

        let (tx, rx) = mpsc::channel();
        migrate_v2_settings_and_servers(tx);

        let info = rx.recv().unwrap();
        assert_event!(info, Event::Info(..));

        let servers = Servers::try_load().unwrap();
        let lotr = servers.get("lotr").unwrap();
        assert!(lotr.tls);
        assert_eq!(5555, lotr.port);
        assert_eq!(false, servers.get("mymud").unwrap().tls);
        assert_eq!(1234, servers.get("mymud2").unwrap().port);
    }

    fn test_doesnt_migrate_over_existing_settings_file() {
        create_v2_settings_file();
        let mut file = fs::File::create(&Settings::relative_path()).unwrap();
        file.write_all(b"").unwrap();

        let (tx, rx) = mpsc::channel();
        migrate_v2_settings_and_servers(tx);

        let event = rx.recv().unwrap();
        assert_event!(event, Event::Error(..));
    }

    fn test_doesnt_migrate_over_existing_tts_settings_file() {
        create_v2_tts_settings_file();
        let mut file = fs::File::create(&TTSSettings::relative_path()).unwrap();
        file.write_all(b"").unwrap();

        let (tx, rx) = mpsc::channel();
        migrate_v2_settings_and_servers(tx);

        let event = rx.recv().unwrap();
        assert_event!(event, Event::Error(..));
    }

    fn test_doesnt_migrate_over_existing_servers_file() {
        create_v2_servers_file();
        let mut file = fs::File::create(&Servers::relative_path()).unwrap();
        file.write_all(b"").unwrap();

        let (tx, rx) = mpsc::channel();
        migrate_v2_settings_and_servers(tx);

        let event = rx.recv().unwrap();
        assert_event!(event, Event::Error(..));
    }
}
