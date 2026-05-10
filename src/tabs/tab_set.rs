use super::tab::{Tab, TabError, TabInfo, TabOpts, MAIN_TAB};
use crate::model::Line;
use crate::ui::History;
use regex::Regex;

/// Result of routing a single inbound line through the tab set.
///
/// The caller (the main event loop) uses this to decide whether to render
/// the line via `screen.print_output(...)`. When `render_to_screen` is
/// `false`, the line was either gagged from the active tab entirely or only
/// matched non-active tabs — either way, the screen should NOT call
/// `print_output`. The line has already been appended into any inactive
/// matching tabs' histories before this result returns.
#[derive(Debug)]
pub struct RouteResult {
    pub render_to_screen: bool,
    /// `true` if any non-active tab received the line — caller may choose
    /// to redraw the tab indicator.
    pub indicator_dirty: bool,
}

/// Owns all tab metadata and the histories of inactive tabs.
///
/// At any moment exactly one tab is "active" — its [`History`] lives in the
/// [`UserInterface`] (the screen). The rest of the tabs hold their
/// histories inside this struct, in `Tab.history = Some(_)`. On `switch`,
/// the caller exchanges the screen's History with the destination tab's
/// History via [`UserInterface::swap_history`].
///
/// [`History`]: crate::ui::History
/// [`UserInterface`]: crate::ui::UserInterface
/// [`UserInterface::swap_history`]: crate::ui::UserInterface::swap_history
pub struct TabSet {
    /// Insertion-ordered list of tabs. `tabs[0]` is always `main`.
    tabs: Vec<Tab>,
    /// Index into `tabs` for the currently-active tab. The active tab's
    /// `history` field is `None` (the History lives in the screen).
    active: usize,
    /// Set during the take/complete switch protocol — the destination tab's
    /// index, taken in `take_for_switch` and consumed in `complete_switch`.
    pending_dest: Option<usize>,
}

impl Default for TabSet {
    fn default() -> Self {
        Self::new()
    }
}

impl TabSet {
    pub fn new() -> Self {
        let mut main = Tab::new_main();
        // The main tab is active on construction, so its history lives in
        // the screen, not here.
        main.history = None;
        Self {
            tabs: vec![main],
            active: 0,
            pending_dest: None,
        }
    }

    fn idx_of(&self, name: &str) -> Option<usize> {
        self.tabs.iter().position(|t| t.name == name)
    }

    /// Create a new tab. `main` is reserved.
    pub fn create(&mut self, name: &str, opts: TabOpts) -> Result<(), TabError> {
        if name == MAIN_TAB {
            return Err(TabError::Reserved(name.to_string()));
        }
        if self.idx_of(name).is_some() {
            return Err(TabError::AlreadyExists(name.to_string()));
        }
        self.tabs.push(Tab::new(name, opts));
        Ok(())
    }

    /// Add a regex string filter to a tab. Compiled once and stored.
    pub fn add_filter(&mut self, name: &str, pattern: &str) -> Result<(), TabError> {
        let idx = self
            .idx_of(name)
            .ok_or_else(|| TabError::Missing(name.to_string()))?;
        let re = Regex::new(pattern).map_err(|e| TabError::BadRegex {
            name: name.to_string(),
            err: e.to_string(),
        })?;
        self.tabs[idx].filters.push(re);
        Ok(())
    }

    pub fn set_label(&mut self, name: &str, label: &str) -> Result<(), TabError> {
        let idx = self
            .idx_of(name)
            .ok_or_else(|| TabError::Missing(name.to_string()))?;
        self.tabs[idx].label = label.to_string();
        Ok(())
    }

    pub fn active_name(&self) -> &str {
        &self.tabs[self.active].name
    }

    pub fn list(&self) -> Vec<TabInfo> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| TabInfo {
                name: t.name.clone(),
                label: t.label.clone(),
                unread: t.unread,
                active: i == self.active,
            })
            .collect()
    }

    /// Decide if and where the line should appear, append it to inactive
    /// matching tabs' histories, and return whether the active tab also
    /// received it (i.e. the screen should render).
    pub fn route(&mut self, line: &Line) -> RouteResult {
        let clean = line.clean_line();

        // First pass: figure out membership without mutating, so we can decide
        // gag-main and active-receipt up front.
        let mut any_gag_main = false;
        let mut active_matches = false;
        // Collect non-active tab indices that matched.
        let mut inactive_matched: Vec<usize> = Vec::new();
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.name == MAIN_TAB {
                continue;
            }
            if tab.matches(clean) {
                if tab.gag_main {
                    any_gag_main = true;
                }
                if i == self.active {
                    active_matches = true;
                } else {
                    inactive_matched.push(i);
                }
            }
        }

        // Main tab receives the line unless every matching tab says gag_main.
        // (Spec: any single matching tab with gag_main=true suppresses main.)
        let main_receives = !any_gag_main;

        // Append to inactive matching tabs' histories.
        let mut indicator_dirty = false;
        for i in inactive_matched {
            if let Some(hist) = self.tabs[i].history.as_mut() {
                hist.append_line(line.clone());
            }
            self.tabs[i].unread = self.tabs[i].unread.saturating_add(1);
            indicator_dirty = true;
        }

        // Append to main if the active tab is NOT main and main receives.
        let main_idx = 0;
        if main_receives && self.active != main_idx {
            if let Some(hist) = self.tabs[main_idx].history.as_mut() {
                hist.append_line(line.clone());
            }
            self.tabs[main_idx].unread = self.tabs[main_idx].unread.saturating_add(1);
            indicator_dirty = true;
        }

        // Decide screen render: active tab receives if it's a matching tab,
        // or if the active tab IS main and main receives.
        let render_to_screen = if self.active == main_idx {
            main_receives
        } else {
            active_matches
        };

        RouteResult {
            render_to_screen,
            indicator_dirty,
        }
    }

    /// Append a line to a specific tab regardless of filters. Used by the
    /// `blight.output_to(name, ...)` Lua API.
    ///
    /// Returns `true` if the active tab was the target (screen needs to
    /// render this line via `print_output`), `false` otherwise.
    pub fn output_to(&mut self, name: &str, line: &Line) -> Result<bool, TabError> {
        let idx = self
            .idx_of(name)
            .ok_or_else(|| TabError::Missing(name.to_string()))?;
        if idx == self.active {
            // Caller renders via screen.print_output; we don't append here
            // because that's the screen's job.
            return Ok(true);
        }
        if let Some(hist) = self.tabs[idx].history.as_mut() {
            hist.append_line(line.clone());
        }
        self.tabs[idx].unread = self.tabs[idx].unread.saturating_add(1);
        Ok(false)
    }

    /// Switch the active tab to `name`. Returns the History to install in
    /// the screen via `screen.swap_history(returned)` AND takes ownership
    /// of the History the caller surrenders (the previously-active one).
    ///
    /// Caller protocol:
    /// ```ignore
    /// // 1. Take the destination tab's history out of the set.
    /// let new_hist = tab_set.take_for_switch(new_name)?;
    /// // 2. Hand it to the screen, receiving the old active history back.
    /// let old_hist = screen.swap_history(new_hist);
    /// // 3. Hand the old back to the set so it can re-install it.
    /// tab_set.complete_switch(old_hist);
    /// ```
    pub fn take_for_switch(&mut self, dest: &str) -> Result<Option<History>, TabError> {
        let dest_idx = self
            .idx_of(dest)
            .ok_or_else(|| TabError::Missing(dest.to_string()))?;
        if dest_idx == self.active {
            // No-op: caller is asking to switch to the already-active tab.
            // Skip the swap entirely.
            return Ok(None);
        }
        let new_hist = self.tabs[dest_idx]
            .history
            .take()
            .expect("inactive tab must have a history");
        self.tabs[dest_idx].unread = 0;
        self.pending_dest = Some(dest_idx);
        Ok(Some(new_hist))
    }

    /// Finish a switch sequence. `old_hist` is the History that was in the
    /// screen before the swap; we re-install it under the previously-active
    /// tab name.
    pub fn complete_switch(&mut self, old_hist: History) -> Result<(), TabError> {
        let dest_idx = self
            .pending_dest
            .take()
            .ok_or_else(|| TabError::Missing("no pending switch".to_string()))?;
        // Re-install the old active tab's history (it was in the screen);
        // activate the destination.
        self.tabs[self.active].history = Some(old_hist);
        self.active = dest_idx;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Line;

    fn line(s: &str) -> Line {
        Line::from(s)
    }

    fn fresh() -> TabSet {
        TabSet::new()
    }

    #[test]
    fn main_exists_and_is_active() {
        let ts = fresh();
        assert_eq!(ts.active_name(), MAIN_TAB);
        let info = ts.list();
        assert_eq!(info.len(), 1);
        assert_eq!(info[0].name, MAIN_TAB);
        assert!(info[0].active);
    }

    #[test]
    fn cannot_create_main() {
        let mut ts = fresh();
        assert!(matches!(
            ts.create(MAIN_TAB, TabOpts::default()),
            Err(TabError::Reserved(_))
        ));
    }

    #[test]
    fn create_chat_and_filter() {
        let mut ts = fresh();
        ts.create("chat", TabOpts::default()).unwrap();
        ts.add_filter("chat", "tells you").unwrap();
        let res = ts.route(&line("Bob tells you: hi"));
        // active is main; chat is non-active matched; main mirrored
        assert!(res.render_to_screen, "main should still render the mirror");
        assert!(res.indicator_dirty, "chat got an unread bump");
        let info = ts.list();
        let chat = info.iter().find(|t| t.name == "chat").unwrap();
        assert_eq!(chat.unread, 1);
    }

    #[test]
    fn gag_main_suppresses_screen_when_active_main() {
        let mut ts = fresh();
        ts.create(
            "spam",
            TabOpts {
                gag_main: true,
                ..Default::default()
            },
        )
        .unwrap();
        ts.add_filter("spam", "boring").unwrap();
        let res = ts.route(&line("this is boring text"));
        assert!(!res.render_to_screen, "gag_main hides line from main view");
    }

    #[test]
    fn no_match_does_not_dirty_indicator() {
        let mut ts = fresh();
        ts.create("chat", TabOpts::default()).unwrap();
        ts.add_filter("chat", "tells you").unwrap();
        let res = ts.route(&line("a stormtrooper attacks you"));
        assert!(res.render_to_screen);
        assert!(!res.indicator_dirty);
    }

    #[test]
    fn switching_protocol_round_trip() {
        let mut ts = fresh();
        ts.create("chat", TabOpts::default()).unwrap();
        // Append something to chat's history while it's inactive.
        ts.add_filter("chat", "tells you").unwrap();
        ts.route(&line("Alice tells you: hello"));

        // Pretend the screen has a history; we'll fake it as a fresh History.
        let fake_screen_history = History::new();
        let dest_history = ts.take_for_switch("chat").unwrap().unwrap();
        // The destination history has the routed line.
        assert!(!dest_history.is_empty());

        ts.complete_switch(fake_screen_history).unwrap();
        assert_eq!(ts.active_name(), "chat");
        // Switching back is the same protocol.
        let main_hist = ts.take_for_switch(MAIN_TAB).unwrap().unwrap();
        ts.complete_switch(main_hist).unwrap();
        assert_eq!(ts.active_name(), MAIN_TAB);
    }

    #[test]
    fn switch_to_already_active_is_noop() {
        let mut ts = fresh();
        // main is already active; ask to switch to main.
        let result = ts.take_for_switch(MAIN_TAB).unwrap();
        assert!(result.is_none(), "switching to active tab returns None");
        assert_eq!(ts.active_name(), MAIN_TAB);
    }
}
