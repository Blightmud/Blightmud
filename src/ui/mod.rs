pub use self::{
    ansi::*,
    command::spawn_input_thread,
    command::CommandBuffer,
    headless_screen::HeadlessScreen,
    help_handler::HelpHandler,
    history::History,
    reader_screen::ReaderScreen,
    split_screen::SplitScreen,
    top_area::{TopPrefix, TopPrefixStyle, TopRowBody, TopRowOpts, TopRowSelector},
    ui_wrapper::UiWrapper,
    user_interface::{wrap_line, UserInterface},
};

#[cfg(test)]
pub use self::user_interface::MockUserInterface;

mod ansi;
mod command;
mod headless_screen;
mod help_handler;
mod history;
mod reader_screen;
mod scroll_data;
mod split_screen;
mod top_area;
mod ui_wrapper;
mod user_interface;
