use crate::{event::Event, model::Line, VERSION};

use std::path::Path;
use std::{borrow::Cow, collections::HashMap, fs, sync::mpsc::Sender};

use log::debug;
use mdcat::{ResourceAccess, Settings, TerminalCapabilities, TerminalSize};
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

    pub fn show_help(&self, file: &str) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Drawing helpfile: {}", file);
        self.writer.send(self.parse_helpfile(file))?;
        Ok(())
    }

    /// Load helpfiles from disk in debug mode, from memory otherwise.
    fn file_content(&self, file: &str) -> Cow<str> {
        if cfg!(debug_assertions) {
            Cow::from(
                fs::read_to_string(self.files[file])
                    .unwrap_or_else(|_| panic!("Can't find {}", file)),
            )
        } else {
            Cow::from(self.files[file])
        }
    }

    fn parse_helpfile(&self, file: &str) -> Event {
        if self.files.contains_key(file) {
            let mut md_bytes = vec![];

            let log_path = crate::DATA_DIR.join("logs");
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
                .replace("$CONFIGDIR", config_dir);

            let mut options = Options::empty();
            options.insert(Options::ENABLE_TASKLISTS);
            options.insert(Options::ENABLE_STRIKETHROUGH);

            let parser = Parser::new_ext(&file_content, options);

            // Useless as files are embedded into binary.
            let base_dir = Path::new("/");

            let env = mdcat::Environment::for_local_directory(&base_dir).unwrap();
            if mdcat::push_tty(&md_settings(), &env, &mut md_bytes, parser).is_ok() {
                if let Ok(md_string) = String::from_utf8(md_bytes) {
                    Event::Output(Line::from(format!("\n\n{}", md_string)))
                } else {
                    Event::Info("Error parsing help file".to_string())
                }
            } else {
                Event::Info("Error parsing help file".to_string())
            }
        } else {
            Event::Info("No such help file found".to_string())
        }
    }
}

fn md_settings() -> Settings {
    let terminal_size = TerminalSize::detect().unwrap_or_default();

    Settings {
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
        "aliases" => "lua_aliases.md",
        "triggers" => "lua_triggers.md",
        "timers" => "lua_timers.md",
        "gmcp" => "lua_gmcp.md",
        "msdp" => "msdp.md",
        "regex" => "regex.md",
        "line" => "line.md",
        "config_scripts" => "config_scripts.md",
        "scripting" => "scripting.md",
        "settings" => "settings.md",
        "storage" => "lua_storage.md",
        "colors" => "lua_colors.md",
        "tasks" => "lua_tasks.md",
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
            assert!(match handler.parse_helpfile(file) {
                Event::Output(_) => true,
                _ => false,
            });
        }
    }

    #[test]
    fn file_not_present() {
        let handler = handler();
        assert_eq!(
            handler.parse_helpfile("nothing"),
            Event::Info("No such help file found".to_string())
        );
    }

    #[test]
    fn confirm_help_render() {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let handler = HelpHandler::new(writer);
        handler.show_help("nothing").unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::Info("No such help file found".to_string()))
        );
        handler.show_help("help").unwrap();
        let line = if let Ok(Event::Output(line)) = reader.recv() {
            Some(line)
        } else {
            None
        };
        assert_ne!(line, None);
        assert!(!line.unwrap().is_empty());
    }
}
