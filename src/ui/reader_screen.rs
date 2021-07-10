use std::io::{stdout, Write};

use anyhow::Result;
use termion::{clear, cursor::Goto, raw::IntoRawMode, screen::AlternateScreen};

use crate::model::{Line, Regex};

use super::{wrap_line, UserInterface};

pub struct ReaderScreen {
    screen: Box<dyn Write>,
    pub width: u16,
    pub height: u16,
}

impl ReaderScreen {
    pub fn new() -> Result<Self> {
        let screen = Box::new(AlternateScreen::from(stdout().into_raw_mode()?));
        let (width, height) = termion::terminal_size()?;
        Ok(Self {
            screen,
            width,
            height,
        })
    }

    pub fn print(&mut self, line: &str, new_line: bool) {
        write!(
            self.screen,
            "{}{}{}{}{}",
            Goto(1, self.height),
            clear::AfterCursor,
            line,
            if new_line { "\n" } else { "" },
            Goto(1, self.height)
        )
        .unwrap();
    }

    pub fn print_line(&mut self, line: &Line) {
        writeln!(
            self.screen,
            "{}{}{}{}",
            Goto(1, self.height),
            clear::AfterCursor,
            line,
            Goto(1, self.height)
        )
        .unwrap();
    }
}

impl UserInterface for ReaderScreen {
    fn setup(&mut self) -> Result<()> {
        Ok(())
    }

    fn print_error(&mut self, output: &str) {
        self.print_line(&Line::from(format!("ERROR: {}", output)));
    }

    fn print_info(&mut self, output: &str) {
        self.print_line(&Line::from(format!("INFO: {}", output)));
    }

    fn print_output(&mut self, line: &Line) {
        if let Some(print_line) = line.print_line() {
            if !line.is_utf8() || print_line.trim().is_empty() {
                self.print(print_line, !line.flags.separate_receives);
            } else {
                let mut new_line = !line.flags.separate_receives;
                for l in wrap_line(print_line, self.width as usize) {
                    self.print(l, new_line);
                    new_line = true;
                }
            }
        }
    }

    fn print_prompt(&mut self, prompt: &Line) {
        self.print_line(&Line::from(format!("{}", prompt)));
    }

    fn print_prompt_input(&mut self, input: &str, pos: usize) {
        write!(
            self.screen,
            "{}{}{}{}",
            Goto(1, self.height),
            clear::AfterCursor,
            input,
            Goto(pos as u16 + 1, self.height)
        )
        .unwrap();
    }

    fn print_send(&mut self, send: &Line) {
        self.print_line(send);
    }

    fn reset(&mut self) -> Result<()> {
        Ok(())
    }

    fn reset_scroll(&mut self) -> Result<()> {
        Ok(())
    }

    fn scroll_down(&mut self) -> Result<()> {
        Ok(())
    }

    fn scroll_lock(&mut self, _lock: bool) -> Result<()> {
        Ok(())
    }

    fn scroll_to(&mut self, _row: usize) -> Result<()> {
        Ok(())
    }

    fn scroll_top(&mut self) -> Result<()> {
        Ok(())
    }

    fn scroll_up(&mut self) -> Result<()> {
        Ok(())
    }

    fn find_up(&mut self, _pattern: &Regex) -> Result<()> {
        Ok(())
    }

    fn find_down(&mut self, _pattern: &Regex) -> Result<()> {
        Ok(())
    }

    fn set_host(&mut self, _host: &str, _port: u16) -> Result<()> {
        Ok(())
    }

    fn set_status_area_height(&mut self, _height: u16) -> Result<()> {
        Ok(())
    }

    fn set_status_line(&mut self, _line: usize, _info: String) -> Result<()> {
        Ok(())
    }

    fn flush(&mut self) {
        self.screen.flush().unwrap();
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }
}
