//! Tabbed scrollable output regions.
//!
//! Provides the data layer (Tab, TabSet, routing) for letting Lua scripts
//! create independently scrollable, named output buffers and bind keys to
//! switch between them. The UI layer borrows the active tab's [`History`]
//! at swap time via [`UserInterface::swap_history`].
//!
//! [`History`]: crate::ui::History
//! [`UserInterface::swap_history`]: crate::ui::UserInterface::swap_history

pub use self::tab::{Tab, TabError, TabInfo, TabOpts};
pub use self::tab_set::{RouteResult, TabSet};

mod tab;
mod tab_set;
