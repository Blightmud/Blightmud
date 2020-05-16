use crate::event::Event;

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
        let output = if self.files.contains_key(file) {
            let mut md_bytes = vec![];

            let file = self.files[file];

            let mut options = Options::empty();
            options.insert(Options::ENABLE_TASKLISTS);
            options.insert(Options::ENABLE_STRIKETHROUGH);

            let parser = Parser::new_ext(&file, options);

            // Useless as files are embedded into binary.
            let base_dir = Path::new("resources/help/");

            if mdcat::push_tty(&md_settings(), &mut md_bytes, &base_dir, parser).is_ok() {
                if let Ok(md_string) = String::from_utf8(md_bytes) {
                    md_string
                } else {
                    "Error parsing help file".to_string()
                }
            } else {
                "Error parsing help file".to_string()
            }
        } else {
            "No such helpfile found".to_string()
        };

        self.writer.send(Event::Info(output)).unwrap();
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
