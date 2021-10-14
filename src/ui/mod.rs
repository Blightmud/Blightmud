pub use self::{
    ansi::*,
    command::spawn_input_thread,
    command::CommandBuffer,
    help_handler::HelpHandler,
    reader_screen::ReaderScreen,
    split_screen::SplitScreen,
    headless_screen::HeadlessScreen,
    ui_wrapper::UiWrapper,
    user_interface::{wrap_line, UserInterface},
};

#[cfg(test)]
pub use self::user_interface::MockUserInterface;

mod ansi;
mod command;
mod help_handler;
mod history;
mod reader_screen;
mod scroll_data;
mod split_screen;
mod headless_screen;
mod ui_wrapper;
mod user_interface;
