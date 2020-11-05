mod connection;
mod line;
mod settings;
pub use connection::{Connection, Servers};
pub use line::Line;
pub use settings::{Settings, LOGGING_ENABLED, MOUSE_ENABLED, SAVE_HISTORY, SETTINGS, TTS_ENABLED};
