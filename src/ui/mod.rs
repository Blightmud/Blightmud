pub use self::{
    ansi::*,
    command::spawn_input_thread,
    command::CommandBuffer,
    headless_screen::HeadlessScreen,
    help_handler::HelpHandler,
    reader_screen::ReaderScreen,
    split_screen::SplitScreen,
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
mod ui_wrapper;
mod user_interface;
