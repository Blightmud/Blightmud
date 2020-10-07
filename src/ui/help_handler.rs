use crate::{event::Event, model::Line, VERSION};

use std::path::Path;
use std::{collections::HashMap, sync::mpsc::Sender};

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

            let file_content = self.files[file]
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

fn load_files() -> HashMap<&'static str, &'static str> {
    let mut files: HashMap<&str, &str> = HashMap::new();
    files.insert("help", include_str!("../../resources/help/help.md"));
    files.insert("changes", include_str!("../../resources/help/changes.md"));
    files.insert("welcome", include_str!("../../resources/help/welcome.md"));
    files.insert("logging", include_str!("../../resources/help/logging.md"));
    files.insert("blight", include_str!("../../resources/help/lua_blight.md"));
    files.insert("bindings", include_str!("../../resources/help/bindings.md"));
    files.insert("core", include_str!("../../resources/help/core.md"));
    files.insert(
        "status_area",
        include_str!("../../resources/help/lua_status_area.md"),
    );
    files.insert(
        "aliases",
        include_str!("../../resources/help/lua_aliases.md"),
    );
    files.insert(
        "triggers",
        include_str!("../../resources/help/lua_triggers.md"),
    );
    files.insert("timers", include_str!("../../resources/help/lua_timers.md"));
    files.insert("gmcp", include_str!("../../resources/help/lua_gmcp.md"));
    files.insert(
        "config_scripts",
        include_str!("../../resources/help/config_scripts.md"),
    );
    files.insert(
        "scripting",
        include_str!("../../resources/help/scripting.md"),
    );
    files.insert("settings", include_str!("../../resources/help/settings.md"));
    files.insert(
        "storage",
        include_str!("../../resources/help/lua_storage.md"),
    );
    files.insert("colors", include_str!("../../resources/help/lua_colors.md"));
    files
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
