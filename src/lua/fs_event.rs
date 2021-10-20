use mlua::{UserData, UserDataFields};

use crate::io::FSEvent as mFsEvent;

#[derive(Clone, Debug)]
pub struct FSEvent {
    pub event: String,
    pub paths: Vec<String>,
}

impl FSEvent {
    pub fn new(event: &str, paths: Vec<Option<&str>>) -> Self {
        Self {
            event: event.to_string(),
            paths: paths.iter().flatten().map(|s| s.to_string()).collect(),
        }
    }

    pub fn ignore() -> Self {
        Self {
            event: "undef".to_string(),
            paths: Vec::new(),
        }
    }
}

impl From<mFsEvent> for FSEvent {
    fn from(src: mFsEvent) -> Self {
        match src {
            mFsEvent::Write(p) => FSEvent::new("write", vec![p.to_str()]),
            mFsEvent::Create(p) => FSEvent::new("create", vec![p.to_str()]),
            mFsEvent::Remove(p) => FSEvent::new("remove", vec![p.to_str()]),
            mFsEvent::Rename(p1, p2) => FSEvent::new("rename", vec![p1.to_str(), p2.to_str()]),
            mFsEvent::Event(p) => FSEvent::new("event", vec![p.to_str()]),
            mFsEvent::Error(_, _) => FSEvent::ignore(),
            mFsEvent::Misc => FSEvent::ignore(),
        }
    }
}

impl UserData for FSEvent {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("event", |_, this| Ok(this.event.clone()));
        fields.add_field_method_get("paths", |_, this| Ok(this.paths.clone()));
    }
}
