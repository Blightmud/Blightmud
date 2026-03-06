use crate::{
    io::SaveData,
    model::{Regex, Settings, SCROLL_LOCK, SCROLL_SPLIT},
};

use super::history::History;
use anyhow::Result;

pub struct ScrollData {
    pub active: bool,
    pub split: bool,
    pub pos: usize,
    pub scroll_lock: bool,
    pub hilite: Option<Regex>,
    pub allow_split: bool,
    pub allow_scroll_lock: bool,
}

impl ScrollData {
    pub fn new() -> Self {
        let settings = Settings::load();
        Self {
            active: false,
            split: false,
            pos: 0,
            scroll_lock: false,
            hilite: None,
            allow_split: settings.get(SCROLL_SPLIT).unwrap_or(true),
            allow_scroll_lock: settings.get(SCROLL_LOCK).unwrap_or(true),
        }
    }

    pub fn reset(&mut self, history: &History) -> Result<()> {
        self.active = false;
        self.split = false;
        self.hilite = None;
        self.pos = if history.is_empty() {
            0
        } else {
            history.len() - 1
        };
        let settings = Settings::try_load()?;
        self.allow_split = settings.get(SCROLL_SPLIT).unwrap_or(true);
        self.allow_scroll_lock = settings.get(SCROLL_LOCK).unwrap_or(true);
        Ok(())
    }

    pub fn lock(&mut self, lock: bool) -> Result<()> {
        self.scroll_lock = lock && self.allow_scroll_lock;
        Ok(())
    }

    pub fn not_scrolled_or_split(&self) -> bool {
        !self.active || self.split
    }

    pub fn clamp(&mut self, history: &History) {
        if self.active {
            while self.pos >= history.len() {
                self.pos -= history.drain_length;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::ui::history::History;

    #[test]
    fn test_general_user() {
        let mut scroll = ScrollData::new();

        assert!(scroll.lock(true).is_ok());
        assert!(scroll.not_scrolled_or_split());
        scroll.active = true;
        scroll.pos = 1000;
        assert!(!scroll.not_scrolled_or_split());

        let history = History::new();
        assert!(scroll.reset(&history).is_ok());
        assert!(scroll.not_scrolled_or_split());
    }

    #[test]
    fn confirm_clamp() {
        let mut scroll = ScrollData::new();
        scroll.active = true;
        scroll.pos = 1200;

        let mut history = History::new();
        for _ in 0..1024 {
            history.append("test")
        }
        assert!(scroll.pos > history.len());
        scroll.clamp(&history);
        assert_eq!(scroll.pos, 1200 - history.drain_length);
    }

    #[test]
    fn test_scroll_data_new_defaults() {
        let scroll = ScrollData::new();
        assert!(!scroll.active);
        assert!(!scroll.split);
        assert_eq!(scroll.pos, 0);
        assert!(!scroll.scroll_lock);
        assert!(scroll.hilite.is_none());
    }

    #[test]
    fn test_scroll_lock_disables_when_not_allowed() {
        let mut scroll = ScrollData::new();
        scroll.allow_scroll_lock = false;
        assert!(scroll.lock(true).is_ok());
        assert!(!scroll.scroll_lock);
    }

    #[test]
    fn test_scroll_lock_enables_when_allowed() {
        let mut scroll = ScrollData::new();
        scroll.allow_scroll_lock = true;
        assert!(scroll.lock(true).is_ok());
        assert!(scroll.scroll_lock);
    }

    #[test]
    fn test_not_scrolled_or_split_when_split() {
        let mut scroll = ScrollData::new();
        scroll.active = true;
        scroll.split = true;
        assert!(scroll.not_scrolled_or_split());
    }

    #[test]
    fn test_not_scrolled_or_split_when_not_active() {
        let scroll = ScrollData::new();
        assert!(scroll.not_scrolled_or_split());
    }

    #[test]
    fn test_clamp_when_not_active() {
        let mut scroll = ScrollData::new();
        scroll.active = false;
        scroll.pos = 1200;

        let mut history = History::new();
        for _ in 0..1024 {
            history.append("test")
        }
        // When not active, clamp should not modify pos
        scroll.clamp(&history);
        assert_eq!(scroll.pos, 1200);
    }

    #[test]
    fn test_reset_with_non_empty_history() {
        let mut scroll = ScrollData::new();
        scroll.active = true;
        scroll.split = true;
        scroll.pos = 500;
        scroll.hilite = Some(Regex::new("test", None).unwrap());

        let mut history = History::new();
        for _ in 0..100 {
            history.append("line");
        }

        assert!(scroll.reset(&history).is_ok());
        assert!(!scroll.active);
        assert!(!scroll.split);
        assert!(scroll.hilite.is_none());
        assert_eq!(scroll.pos, history.len() - 1);
    }

    #[test]
    fn test_reset_with_empty_history() {
        let mut scroll = ScrollData::new();
        scroll.pos = 500;

        let history = History::new();
        assert!(scroll.reset(&history).is_ok());
        assert_eq!(scroll.pos, 0);
    }
}
