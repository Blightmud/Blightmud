use crate::ansi::*;
use std::io::{stdout, Stdout, Write};
use termion::{
    color,
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
};

pub struct Screen {
    screen: AlternateScreen<RawTerminal<Stdout>>,
    width: u16,
    _height: u16,
    output_line: u16,
    prompt_line: u16,
}

impl Screen {
    pub fn new() -> Self {
        let screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        let (width, height) = termion::terminal_size().unwrap();

        let output_line = height - 2;
        let prompt_line = height;

        Self {
            screen,
            width,
            _height: height,
            output_line,
            prompt_line,
        }
    }

    pub fn setup(&mut self) {
        self.reset();
        write!(
            self.screen,
            "{}{}",
            ScrollRegion(2, self.output_line),
            DisableOriginMode
        )
        .unwrap(); // Set scroll region, non origin mode
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, self.output_line + 1),
            termion::clear::AfterCursor,
            color::Bg(color::White),
        )
        .unwrap();
        write!(self.screen, "{: <1$}", "", self.width as usize).unwrap(); // Print separator
        write!(self.screen, "{}", color::Bg(color::Reset)).unwrap();
        self.screen.flush().unwrap();
    }

    pub fn reset(&mut self) {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion).unwrap();
    }

    pub fn print_prompt(&mut self, prompt: &str) {
        write!(
            self.screen,
            "{}{}{}{}",
            termion::cursor::Goto(1, self.output_line),
            termion::scroll::Up(1),
            prompt,
            termion::cursor::Goto(1, self.prompt_line),
        )
        .unwrap();
    }

    pub fn print_prompt_input(&mut self, input: &str) {
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, self.prompt_line),
            termion::clear::AfterCursor,
            input,
        )
        .unwrap();
    }

    pub fn print_prompt_with_input(&mut self, prompt: &str, input: &str) {
        write!(
            self.screen,
            "{}{}{} {}{}{}",
            termion::cursor::Goto(1, self.output_line),
            termion::clear::AfterCursor,
            prompt,
            color::Fg(color::LightYellow),
            input,
            color::Fg(color::Reset),
        )
        .unwrap();
    }

    pub fn print_output(&mut self, output: &str) {
        write!(
            self.screen,
            "{}{}{}{}",
            termion::cursor::Goto(1, self.output_line),
            termion::scroll::Up(1),
            output,
            termion::cursor::Goto(1, self.prompt_line)
        )
        .unwrap();
    }

    pub fn scroll_up(&mut self) {
        write!(self.screen, "{}", termion::scroll::Up(5)).unwrap();
    }

    pub fn scroll_down(&mut self) {
        write!(self.screen, "{}", termion::scroll::Down(5)).unwrap();
    }

    pub fn flush(&mut self) {
        self.screen.flush().unwrap();
    }
}
