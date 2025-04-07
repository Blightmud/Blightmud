use mlua::{UserData, UserDataFields};

use crate::io::FSEvent as mFsEvent;

#[derive(Clone, Debug)]
pub struct FSEvent {
    pub paths: Vec<String>,
}

impl FSEvent {
    pub fn new(paths: Vec<Option<&str>>) -> Self {
        Self {
            paths: paths.iter().flatten().map(|s| s.to_string()).collect(),
        }
    }

    pub fn ignore() -> Self {
        Self { paths: Vec::new() }
    }
}

impl From<mFsEvent> for FSEvent {
    fn from(src: mFsEvent) -> Self {
        match src {
            mFsEvent::Update(paths) => {
                FSEvent::new(paths.iter().map(|p| p.to_str().to_owned()).collect())
            }
            mFsEvent::Error(_, _) => FSEvent::ignore(),
        }
    }
}

impl UserData for FSEvent {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("paths", |_, this| Ok(this.paths.clone()));
    }
}
