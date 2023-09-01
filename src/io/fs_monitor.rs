use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    time::Duration,
};

use notify_debouncer_mini::{
    new_debouncer,
    notify::{self, RecommendedWatcher},
    DebounceEventResult, Debouncer,
};

use crate::event::Event;
use std::io::Result;

pub struct FSMonitor {
    watcher: Debouncer<RecommendedWatcher>,
}

impl FSMonitor {
    pub fn new(main_writer: Sender<Event>) -> Result<Self> {
        let watcher = new_debouncer(Duration::from_secs(5), move |res: DebounceEventResult| {
            main_writer
                .send(Event::FSEvent(FSEvent::from(res)))
                .unwrap();
        })
        .unwrap();

        Ok(Self { watcher })
    }

    pub fn watch(&mut self, p: &Path) -> notify::Result<()> {
        self.watcher
            .watcher()
            .watch(p, notify::RecursiveMode::Recursive)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FSEvent {
    Update(Vec<PathBuf>),
    Error(String, Option<PathBuf>),
}

impl From<DebounceEventResult> for FSEvent {
    fn from(res: DebounceEventResult) -> Self {
        match res {
            Ok(events) => FSEvent::Update(events.iter().map(|e| e.path.to_owned()).collect()),
            Err(errors) => FSEvent::Error(format!("{errors:#?}"), None),
        }
    }
}
