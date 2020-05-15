use crate::event::Event;
use std::{collections::HashMap, sync::mpsc::Sender};

pub struct HelpHandler {
    writer: Sender<Event>,
    files: HashMap<&'static str, String>,
}

impl HelpHandler {
    pub fn new(writer: Sender<Event>) -> Self {
        let files = load_files();
        Self { writer, files }
    }

    pub fn show_help(&self, file: &str) {
        if self.files.contains_key(file) {
            self.writer
                .send(Event::MudOutput(self.files[file].clone()))
                .unwrap();
        } else {
            self.writer
                .send(Event::Info("No such helpfile found".to_string()))
                .unwrap();
        }
    }
}

fn load_files() -> HashMap<&'static str, String> {
    let mut files: HashMap<&str, String> = HashMap::new();
    files.insert(
        "help",
        String::from_utf8(include_bytes!("../resources/help/help.md").to_vec()).unwrap(),
    );
    files.insert(
        "welcome",
        String::from_utf8(include_bytes!("../resources/help/welcome.md").to_vec()).unwrap(),
    );
    files.insert(
        "scripting",
        String::from_utf8(include_bytes!("../resources/help/scripting.md").to_vec()).unwrap(),
    );
    files
}
