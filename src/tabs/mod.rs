//! Tabbed scrollable output regions.
//!
//! Provides the data layer (Tab, TabSet, routing) for letting Lua scripts
//! create independently scrollable, named output buffers and bind keys to
//! switch between them. The UI layer borrows the active tab's [`History`]
//! at swap time via [`UserInterface::swap_history`].
//!
//! [`History`]: crate::ui::History
//! [`UserInterface::swap_history`]: crate::ui::UserInterface::swap_history

// Public API:
//   TabOpts  — configuration passed to `blight.create_tab` (carried by Event::TabCommand)
//   TabSet   — owned by Session; routes inbound lines + holds inactive tabs' Histories
//   TabInfo  — read-only snapshot used by the screen's tab indicator and Lua introspection
//   MAIN_TAB — reserved name of the implicit default tab (used by the event layer's
//              remove/switch fallback logic)
//
// `Tab`, `TabError`, and `RouteResult` are intentionally not re-exported here:
// they're used only inside this module (and via TabSet's public methods that
// happen to return them — those types are reachable through return-type
// inference at call sites without needing a direct import).
pub use self::tab::{TabInfo, TabOpts, MAIN_TAB};
pub use self::tab_set::TabSet;

mod tab;
mod tab_set;
