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
}
