use crate::{event::Event, VERSION};

use std::path::Path;
use std::{collections::HashMap, sync::mpsc::Sender};

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

    pub fn show_help(&self, file: &str) {
        self.writer.send(self.parse_helpfile(file)).unwrap();
    }

    fn parse_helpfile(&self, file: &str) -> Event {
        if self.files.contains_key(file) {
            let mut md_bytes = vec![];

            let file = self.files[file].replace("$VERSION", VERSION);

            let mut options = Options::empty();
            options.insert(Options::ENABLE_TASKLISTS);
            options.insert(Options::ENABLE_STRIKETHROUGH);

            let parser = Parser::new_ext(&file, options);

            // Useless as files are embedded into binary.
            let base_dir = Path::new("resources/help/");

            if mdcat::push_tty(&md_settings(), &mut md_bytes, &base_dir, parser).is_ok() {
                if let Ok(md_string) = String::from_utf8(md_bytes) {
                    Event::Output(md_string)
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
    files.insert("help", include_str!("../resources/help/help.md"));
    files.insert("welcome", include_str!("../resources/help/welcome.md"));
    files.insert("scripting", include_str!("../resources/help/scripting.md"));
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
}
