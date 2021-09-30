use crate::{event::Event, model::Line, VERSION};

use std::path::{Path, PathBuf};
use std::{borrow::Cow, collections::HashMap, fs, sync::mpsc::Sender};

use log::debug;
use mdcat::{ResourceAccess, Settings as MDSettings, TerminalCapabilities, TerminalSize};
use pulldown_cmark::{Options, Parser};
use syntect::parsing::SyntaxSet;

pub struct HelpHandler {
    writer: Sender<Event>,
    files: HashMap<&'static str, &'static str>,
}

impl HelpHandler {
    pub fn new(writer: Sender<Event>) -> Self {
        let files = load_files();
        Self { writer, files }
    }

    pub fn show_help(&self, file: &str, lock: bool) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Drawing help file: {}", file);
        if lock {
            self.writer.send(Event::ScrollLock(true))?;
        }
        if let Some(line) = self.parse_helpfile(file) {
            self.writer.send(Event::Output(line)).unwrap();
        } else if let Some(line) = self.search_helpfiles(file) {
            self.writer.send(Event::Output(line)).unwrap();
        } else {
            self.writer
                .send(Event::Info("No help files found".to_string()))
                .unwrap();
        }
        if lock {
            self.writer.send(Event::ScrollLock(false))?;
        }
        Ok(())
    }

    fn read_from_file(&self, file: &str) -> Cow<str> {
        Cow::from(fs::read_to_string(file).unwrap_or_else(|_| panic!("Can't find {}", file)))
    }

    /// Load helpfiles from disk in debug mode, from memory otherwise.
    fn file_content(&self, file: &str) -> Cow<str> {
        if cfg!(debug_assertions) {
            self.read_from_file(self.files[file])
        } else {
            Cow::from(self.files[file])
        }
    }

    fn get_plugin_helpfile_path(&self, file: &str) -> PathBuf {
        crate::DATA_DIR.join("plugins").join(file).join("README.md")
    }

    fn parse_markdown(&self, file_content: &str) -> Option<Line> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TASKLISTS);
        options.insert(Options::ENABLE_STRIKETHROUGH);

        let parser = Parser::new_ext(file_content, options);

        // Useless as files are embedded into binary.
        let base_dir = Path::new("/");

        let mut md_bytes = vec![];
        let env = mdcat::Environment::for_local_directory(&base_dir).unwrap();
        if mdcat::push_tty(&md_settings(), &env, &mut md_bytes, parser).is_ok() {
            if let Ok(md_string) = String::from_utf8(md_bytes) {
                Some(Line::from(format!("\n\n{}", md_string)))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_helpfile(&self, file: &str) -> Option<Line> {
        let plugin_help_path = self.get_plugin_helpfile_path(file);
        if plugin_help_path.exists() {
            if let Some(path) = plugin_help_path.to_str() {
                let content = self.read_from_file(path);
                self.parse_markdown(&content)
            } else {
                None
            }
        } else if self.files.contains_key(file) {
            let data_dir = crate::DATA_DIR.clone();
            let log_path = data_dir.join("logs");
            let datadir = if let Some(str_path) = data_dir.to_str() {
                str_path
            } else {
                "$USER_DATA_DIR"
            };
            let logdir = if let Some(str_path) = log_path.to_str() {
                str_path
            } else {
                "$USER_DATA_DIR/logs"
            };
            let config_path = crate::CONFIG_DIR.to_path_buf();
            let config_dir = if let Some(str_path) = config_path.to_str() {
                str_path
            } else {
                "$USER_CONFIG_DIR"
            };

            let file_content = self
                .file_content(file)
                .replace("$VERSION", VERSION)
                .replace("$LOGDIR", logdir)
                .replace("$DATADIR", datadir)
                .replace("$CONFIGDIR", config_dir);

            self.parse_markdown(&file_content)
        } else {
            None
        }
    }

    pub fn search_helpfiles(&self, pattern: &str) -> Option<Line> {
        let mut matches = vec![];
        for key in self.files.keys() {
            let content = self.file_content(key);
            if content.contains(pattern) {
                matches.push(key);
            }
        }
        if !matches.is_empty() {
            let mut output = "No such help file exists.\nThe following help files contain a match for your search:".to_string();
            for key in matches {
                output.push_str(&format!("\n- {}", key));
            }
            Some(Line::from(output))
        } else {
            None
        }
    }
}

fn md_settings() -> MDSettings {
    let terminal_size = TerminalSize::detect().unwrap_or_default();

    MDSettings {
        terminal_capabilities: TerminalCapabilities::ansi(),
        terminal_size,
        resource_access: ResourceAccess::LocalOnly,
        syntax_set: SyntaxSet::load_defaults_newlines(),
    }
}

macro_rules! help_files {
    ($(
        $(#[$attr:meta])*
        $name:literal => $path:literal,
    )+) => {
        let mut files: HashMap<&str, &str> = HashMap::new();
        $(
            $(#[$attr])*
            files.insert(
                $name,
                if cfg!(debug_assertions) {
                    concat!(env!("CARGO_MANIFEST_DIR"), "/resources/help/", $path)
                } else {
                    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/help/", $path))
                }
            );
        )+
        files
    };
    // Same as above but allows no trailing comma.
    ($($(#[$attr:meta])* $file:literal => $path:literal),+) => {
        help_files!($($(#[$attr])* $file => $path,)+)
    };
}

fn load_files() -> HashMap<&'static str, &'static str> {
    help_files! {
        "help" => "help.md",
        "changes" => "changes.md",
        "welcome" => "welcome.md",
        "logging" => "logging.md",
        "blight" => "lua_blight.md",
        "bindings" => "bindings.md",
        "core" => "core.md",
        #[cfg(feature = "tts")]
        "tts" => "tts.md",
        #[cfg(not(feature = "tts"))]
        "tts" => "no_tts.md",
        "status_area" => "lua_status_area.md",
        "alias" => "lua_aliases.md",
        "config" => "lua_settings.md",
        "script" => "script.md",
        "trigger" => "lua_trigger.md",
        "timers" => "lua_timers.md",
        "gmcp" => "lua_gmcp.md",
        "msdp" => "msdp.md",
        "regex" => "regex.md",
        "line" => "line.md",
        "mud" => "mud.md",
        "audio" => "audio.md",
        "log" => "log.md",
        "config_scripts" => "config_scripts.md",
        "scripting" => "scripting.md",
        "settings" => "settings.md",
        "storage" => "lua_storage.md",
        "colors" => "lua_colors.md",
        "tasks" => "lua_tasks.md",
        "socket" => "socket.md",
        "plugin" => "plugin.md",
        "plugin_developer" => "plugin_developer.md",
        "servers" => "servers.md",
        "search" => "search.md",
        "scrolling" => "scrolling.md",
        "ttype" => "ttype.md",
        "json" => "json.md",
        "prompt" => "prompt.md",
        "history" => "history.md",
    }
}

#[cfg(test)]
mod help_test {

    use super::HelpHandler;
    use crate::event::Event;
    use std::sync::mpsc::{channel, Receiver, Sender};

    fn handler() -> HelpHandler {
        let (writer, _): (Sender<Event>, Receiver<Event>) = channel();
        HelpHandler::new(writer)
    }

    #[test]
    fn confirm_markdown_parsing() {
        let handler = handler();
        for file in handler.files.keys() {
            assert!(handler.parse_helpfile(file).is_some());
        }
    }

    #[test]
    fn file_not_present() {
        let handler = handler();
        assert_eq!(handler.parse_helpfile("nothing"), None);
    }

    #[test]
    fn confirm_help_render() {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let handler = HelpHandler::new(writer);
        handler
            .show_help("defintitelydoesntmatchanything", false)
            .unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::Info("No help files found".to_string()))
        );
        handler.show_help("help", false).unwrap();
        let line = if let Ok(Event::Output(line)) = reader.recv() {
            Some(line)
        } else {
            None
        };
        assert_ne!(line, None);
        assert!(!line.unwrap().is_empty());
    }
}
