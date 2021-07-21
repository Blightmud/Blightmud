pub use self::{
    ansi::*,
    command::spawn_input_thread,
    help_handler::HelpHandler,
    reader_screen::ReaderScreen,
    screen::{create_screen, create_screen_writer, switch_screen},
    split_screen::SplitScreen,
    user_interface::{wrap_line, UserInterface},
};

#[cfg(test)]
pub use self::user_interface::MockUserInterface;

mod ansi;
mod command;
mod help_handler;
mod history;
mod reader_screen;
mod screen;
mod scroll_data;
mod split_screen;
mod user_interface;
