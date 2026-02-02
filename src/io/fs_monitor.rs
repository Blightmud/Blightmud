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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_event_update_empty() {
        let event = FSEvent::Update(vec![]);
        match event {
            FSEvent::Update(paths) => assert!(paths.is_empty()),
            _ => panic!("Expected Update variant"),
        }
    }

    #[test]
    fn test_fs_event_update_with_paths() {
        let paths = vec![PathBuf::from("/test/path"), PathBuf::from("/another/path")];
        let event = FSEvent::Update(paths.clone());
        match event {
            FSEvent::Update(p) => assert_eq!(p, paths),
            _ => panic!("Expected Update variant"),
        }
    }

    #[test]
    fn test_fs_event_error_with_path() {
        let event = FSEvent::Error("test error".to_string(), Some(PathBuf::from("/error/path")));
        match event {
            FSEvent::Error(msg, path) => {
                assert_eq!(msg, "test error");
                assert_eq!(path, Some(PathBuf::from("/error/path")));
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_fs_event_error_without_path() {
        let event = FSEvent::Error("test error".to_string(), None);
        match event {
            FSEvent::Error(msg, path) => {
                assert_eq!(msg, "test error");
                assert!(path.is_none());
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_fs_event_clone() {
        let event = FSEvent::Update(vec![PathBuf::from("/test")]);
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    #[test]
    fn test_fs_event_equality() {
        let event1 = FSEvent::Update(vec![PathBuf::from("/path1")]);
        let event2 = FSEvent::Update(vec![PathBuf::from("/path1")]);
        assert_eq!(event1, event2);
    }

    #[test]
    fn test_fs_event_inequality() {
        let event1 = FSEvent::Update(vec![PathBuf::from("/path1")]);
        let event2 = FSEvent::Update(vec![PathBuf::from("/path2")]);
        assert_ne!(event1, event2);
    }

    #[test]
    fn test_fs_event_debug() {
        let event = FSEvent::Update(vec![PathBuf::from("/debug/path")]);
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Update"));
        assert!(debug_str.contains("/debug/path"));
    }
}
