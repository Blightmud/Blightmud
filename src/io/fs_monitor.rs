use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use log::{debug, error};
use notify::{watcher, DebouncedEvent, RecommendedWatcher};

use crate::event::Event;
use std::io::Result;

pub struct FSMonitor {
    watcher: RecommendedWatcher,
    _thread: JoinHandle<()>,
}

impl Deref for FSMonitor {
    type Target = RecommendedWatcher;

    fn deref(&self) -> &Self::Target {
        &self.watcher
    }
}

impl DerefMut for FSMonitor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.watcher
    }
}

impl FSMonitor {
    pub fn new(main_writer: Sender<Event>) -> Result<Self> {
        let (tx, rx) = channel();
        let watcher = watcher(tx, Duration::from_secs(5)).unwrap();
        let thread = spawn_monitor_thread(main_writer, rx)?;

        Ok(Self {
            watcher,
            _thread: thread,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum FSEvent {
    Write(PathBuf),
    Create(PathBuf),
    Remove(PathBuf),
    Rename(PathBuf, PathBuf),
    Event(PathBuf),
    Error(String, Option<PathBuf>),
    Misc,
}

impl From<DebouncedEvent> for FSEvent {
    fn from(e: DebouncedEvent) -> Self {
        if let DebouncedEvent::Error(_, _) = e {
            error!("[FSMON]: {:?}", e)
        } else {
            debug!("[FSMON]: {:?}", e)
        }
        match e {
            DebouncedEvent::Create(path) => FSEvent::Create(path),
            DebouncedEvent::Write(path) => FSEvent::Write(path),
            DebouncedEvent::Remove(path) => FSEvent::Remove(path),
            DebouncedEvent::Rename(from, to) => FSEvent::Rename(from, to),
            DebouncedEvent::NoticeRemove(path) | DebouncedEvent::NoticeWrite(path) => {
                FSEvent::Event(path)
            }
            DebouncedEvent::Error(err, path) => FSEvent::Error(format!("{:?}", err), path),
            _ => FSEvent::Misc,
        }
    }
}

pub fn spawn_monitor_thread(
    main_writer: Sender<Event>,
    rx: Receiver<DebouncedEvent>,
) -> Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("fs-monitor-thread".to_string())
        .spawn(move || {
            while let Ok(event) = rx.recv() {
                main_writer
                    .send(Event::FSEvent(FSEvent::from(event)))
                    .unwrap();
            }
        })
}
