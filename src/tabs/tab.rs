use crate::ui::History;
use regex::Regex;
use std::fmt;

/// Reserved name for the implicit default tab. Always exists, always
/// receives non-gagged output, can be `switch_tab`'d to.
pub const MAIN_TAB: &str = "main";

/// Per-tab configuration the user can supply at create time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabOpts {
    /// Display label rendered on the tab indicator. Falls back to the tab
    /// name if `None`.
    pub label: Option<String>,
    /// Optional keyboard-shortcut hint rendered in the tab indicator. When
    /// `Some("F2")`, the tab is shown as `[F2 - chat]` (active) or
    /// `(F2 - chat·3)` (inactive with unread). This is a *display* hint
    /// only — Blightmud does not bind the key for you. Use `blight.bind`
    /// in tandem (see `/help tabs`).
    pub shortcut: Option<String>,
    /// When `true`, lines that match THIS tab's filter are NOT mirrored to
    /// the `main` tab — they appear ONLY in this tab's scrollback. Default
    /// `false` (mirror).
    pub gag_main: bool,
    /// Approximate scrollback capacity for this tab, in lines. `None` keeps
    /// the same depth as the `main` scrollback (~32k lines). A smaller value
    /// bounds the tab's peak memory — e.g. `Some(2000)` caps a busy tab to
    /// ~2k lines. Mapped to the history's drain length as `(lines / 32).max(1)`.
    pub history_lines: Option<usize>,
}

/// Errors returned from [`TabSet`] mutation operations.
///
/// [`TabSet`]: super::TabSet
#[derive(Debug)]
pub enum TabError {
    /// A tab with the given name is already registered.
    AlreadyExists(String),
    /// No tab with the given name exists.
    Missing(String),
    /// Reserved tab name (`main`) cannot be created or removed by scripts.
    Reserved(String),
    /// A regex pattern provided by the script failed to compile.
    BadRegex { name: String, err: String },
}

impl fmt::Display for TabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TabError::AlreadyExists(name) => write!(f, "tab already exists: {name}"),
            TabError::Missing(name) => write!(f, "no such tab: {name}"),
            TabError::Reserved(name) => write!(f, "tab name is reserved: {name}"),
            TabError::BadRegex { name, err } => {
                write!(f, "bad regex for tab `{name}`: {err}")
            }
        }
    }
}

impl std::error::Error for TabError {}

/// Snapshot of a tab's user-visible state. Returned from
/// [`TabSet::list`] for use by the renderer / Lua introspection.
///
/// [`TabSet::list`]: super::TabSet::list
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub name: String,
    pub label: String,
    /// Display-only keyboard hint (e.g. `"F2"`). `None` when unset.
    pub shortcut: Option<String>,
    pub unread: u32,
    pub active: bool,
}

/// A single named tab.
///
/// The `history` field is `Some` when this tab is **inactive** (its
/// scrollback lives in this struct). When the tab is **active**, ownership
/// of the History is held by the [`UserInterface`], and this field is
/// `None`. Tab metadata (label, filters, unread counter, etc.) always lives
/// here regardless of active/inactive state.
///
/// [`UserInterface`]: crate::ui::UserInterface
pub struct Tab {
    pub name: String,
    pub label: String,
    /// Optional keyboard-shortcut hint shown in the tab indicator.
    pub shortcut: Option<String>,
    pub gag_main: bool,
    pub filters: Vec<Regex>,
    /// Regex patterns that, when matched, veto routing into this tab even
    /// if a `filters` pattern also matches. Used to carve narrow holes in
    /// a broad include rule (e.g. include `^[A-Z]\w+ says\b` but exclude
    /// `^(He|She|Smuggler) says\b`). Rust's `regex` crate has no
    /// lookaround, so exclusions are expressed as a second list rather
    /// than inline.
    pub excludes: Vec<Regex>,
    pub history: Option<History>,
    pub unread: u32,
}

impl Tab {
    pub fn new(name: impl Into<String>, opts: TabOpts) -> Self {
        let name = name.into();
        let label = opts.label.unwrap_or_else(|| name.clone());
        // `history_lines` is an approximate line capacity; a History retains
        // ~`32 * drain_length` lines, so map lines -> drain_length. `None`
        // defaults to 1024 (≈32k-line depth, matching the main scrollback).
        // `for_tab` grows the backing store on demand instead of reserving it
        // up front, so an idle tab costs ~zero heap regardless of depth.
        let drain_length = opts.history_lines.map(|n| (n / 32).max(1)).unwrap_or(1024);
        Self {
            name,
            label,
            shortcut: opts.shortcut,
            gag_main: opts.gag_main,
            filters: Vec::new(),
            excludes: Vec::new(),
            history: Some(History::for_tab(drain_length)),
            unread: 0,
        }
    }

    /// Create the implicit `main` tab.
    pub fn new_main() -> Self {
        Self::new(
            MAIN_TAB,
            TabOpts {
                label: Some(MAIN_TAB.to_string()),
                shortcut: None,
                gag_main: false,
                history_lines: None,
            },
        )
    }

    /// `true` when any of this tab's regex filters match the line AND no
    /// exclude pattern vetoes it. The `main` tab returns `false` here (it
    /// doesn't have filters — it's the mirror, and routing handles main
    /// as a special case).
    pub fn matches(&self, clean_line: &str) -> bool {
        if self.excludes.iter().any(|re| re.is_match(clean_line)) {
            return false;
        }
        self.filters.iter().any(|re| re.is_match(clean_line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_error_display_messages() {
        assert_eq!(
            TabError::AlreadyExists("chat".into()).to_string(),
            "tab already exists: chat"
        );
        assert_eq!(TabError::Missing("x".into()).to_string(), "no such tab: x");
        assert_eq!(
            TabError::Reserved("main".into()).to_string(),
            "tab name is reserved: main"
        );
        assert_eq!(
            TabError::BadRegex {
                name: "chat".into(),
                err: "oops".into()
            }
            .to_string(),
            "bad regex for tab `chat`: oops"
        );
    }

    #[test]
    fn new_tab_label_falls_back_to_name() {
        let plain = Tab::new("chat", TabOpts::default());
        assert_eq!(plain.label, "chat");
        assert!(plain.history.is_some());
        assert_eq!(plain.unread, 0);
        assert!(plain.shortcut.is_none());

        let labeled = Tab::new(
            "chat",
            TabOpts {
                label: Some("Chat".into()),
                shortcut: Some("F2".into()),
                gag_main: true,
                history_lines: None,
            },
        );
        assert_eq!(labeled.label, "Chat");
        assert_eq!(labeled.shortcut, Some("F2".into()));
        assert!(labeled.gag_main);
    }

    #[test]
    fn main_tab_defaults() {
        let main = Tab::new_main();
        assert_eq!(main.name, MAIN_TAB);
        assert_eq!(main.label, MAIN_TAB);
        assert!(!main.gag_main);
    }

    #[test]
    fn matches_respects_filters_and_excludes() {
        let mut tab = Tab::new("chat", TabOpts::default());
        // No filters → nothing matches.
        assert!(!tab.matches("Bob tells you: hi"));
        tab.filters.push(Regex::new("tells you").unwrap());
        assert!(tab.matches("Bob tells you: hi"));
        assert!(!tab.matches("nothing here"));
        // An exclude vetoes an otherwise-matching line.
        tab.excludes.push(Regex::new("^Bob").unwrap());
        assert!(!tab.matches("Bob tells you: hi"));
    }

    #[test]
    fn history_lines_maps_to_capacity() {
        let tab = Tab::new(
            "chat",
            TabOpts {
                history_lines: Some(2000),
                ..Default::default()
            },
        );
        let h = tab.history.as_ref().expect("inactive tab has a history");
        // 2000 / 32 = 62 -> capacity 32 * 62 = 1984.
        assert_eq!(h.drain_length, 62);
        assert_eq!(h.capacity, 1984);
    }

    #[test]
    fn default_tab_keeps_main_depth() {
        let tab = Tab::new("chat", TabOpts::default());
        let h = tab.history.as_ref().expect("inactive tab has a history");
        // No history_lines -> same drain ceiling as the main scrollback.
        assert_eq!(h.drain_length, 1024);
        assert_eq!(h.capacity, 32 * 1024);
    }

    #[test]
    fn tiny_history_lines_clamps_to_one_drain() {
        let tab = Tab::new(
            "chat",
            TabOpts {
                history_lines: Some(5),
                ..Default::default()
            },
        );
        let h = tab.history.as_ref().unwrap();
        // 5 / 32 = 0 -> clamped to drain_length 1, capacity 32.
        assert_eq!(h.drain_length, 1);
        assert_eq!(h.capacity, 32);
    }
}
