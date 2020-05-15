use crate::event::Event;
use std::{collections::HashMap, sync::mpsc::Sender};

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
        if self.files.contains_key(file) {
            self.writer
                .send(Event::MudOutput(self.files[file].to_string()))
                .unwrap();
        } else {
            self.writer
                .send(Event::Info("No such helpfile found".to_string()))
                .unwrap();
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
