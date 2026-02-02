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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_fs_event_new() {
        let event = FSEvent::new(vec![Some("/path/to/file"), Some("/another/path")]);
        assert_eq!(event.paths, vec!["/path/to/file", "/another/path"]);
    }

    #[test]
    fn test_fs_event_new_with_none() {
        let event = FSEvent::new(vec![Some("/path/to/file"), None, Some("/another/path")]);
        assert_eq!(event.paths, vec!["/path/to/file", "/another/path"]);
    }

    #[test]
    fn test_fs_event_new_empty() {
        let event = FSEvent::new(vec![]);
        assert!(event.paths.is_empty());
    }

    #[test]
    fn test_fs_event_ignore() {
        let event = FSEvent::ignore();
        assert!(event.paths.is_empty());
    }

    #[test]
    fn test_fs_event_from_update() {
        let paths = vec![PathBuf::from("/test/path"), PathBuf::from("/other/path")];
        let m_event = mFsEvent::Update(paths);
        let event: FSEvent = m_event.into();
        assert_eq!(event.paths, vec!["/test/path", "/other/path"]);
    }

    #[test]
    fn test_fs_event_from_error() {
        let m_event = mFsEvent::Error("some error".to_string(), None);
        let event: FSEvent = m_event.into();
        assert!(event.paths.is_empty());
    }

    #[test]
    fn test_fs_event_clone() {
        let event = FSEvent::new(vec![Some("/path/to/file")]);
        let cloned = event.clone();
        assert_eq!(event.paths, cloned.paths);
    }
}
