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
    /// When `true`, lines that match THIS tab's filter are NOT mirrored to
    /// the `main` tab — they appear ONLY in this tab's scrollback. Default
    /// `false` (mirror).
    pub gag_main: bool,
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
        Self {
            name,
            label,
            gag_main: opts.gag_main,
            filters: Vec::new(),
            excludes: Vec::new(),
            history: Some(History::new()),
            unread: 0,
        }
    }

    /// Create the implicit `main` tab.
    pub fn new_main() -> Self {
        Self::new(
            MAIN_TAB,
            TabOpts {
                label: Some(MAIN_TAB.to_string()),
                gag_main: false,
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
