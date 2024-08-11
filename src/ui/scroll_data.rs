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
}
