use std::io::Write;

use anyhow::Result;
use termion::{clear, cursor::Goto};

use crate::model::{Line, Regex};

use super::{history::History, scroll_data::ScrollData, wrap_line, UserInterface};

pub struct ReaderScreen {
    screen: Box<dyn Write>,
    history: History,
    scroll_data: ScrollData,
    pub width: u16,
    pub height: u16,
}

impl ReaderScreen {
    pub fn new(screen: Box<dyn Write>, history: History) -> Result<Self> {
        let (width, height) = termion::terminal_size()?;
        let scroll_data = ScrollData::new();
        Ok(Self {
            screen,
            history,
            scroll_data,
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

    fn draw_scroll(&mut self) -> Result<()> {
        for i in 0..self.height - 1 {
            let index = self.scroll_data.pos + i as usize;
            let line = self.history.inner[index].clone();
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, i + 1),
                termion::clear::CurrentLine,
                termion::style::Reset,
                line,
            )?;
        }
        Ok(())
    }
}

impl UserInterface for ReaderScreen {
    fn setup(&mut self) -> Result<()> {
        self.reset()?;
        self.reset_scroll()?;
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
        write!(self.screen, "{}", termion::clear::All)?;
        Ok(())
    }

    fn reset_scroll(&mut self) -> Result<()> {
        self.scroll_data.reset(&self.history)?;
        let output_range = self.height - 1;
        let output_start_index = self.history.inner.len() as i32 - output_range as i32;
        if output_start_index >= 0 {
            let output_start_index = output_start_index as usize;
            for i in 0..output_range {
                let index = output_start_index + i as usize;
                let line_no = i + 1;
                write!(
                    self.screen,
                    "{}{}{}",
                    termion::cursor::Goto(1, line_no),
                    termion::clear::CurrentLine,
                    self.history.inner[index],
                )?;
            }
        } else {
            for line in &self.history.inner {
                write!(
                    self.screen,
                    "{}\n{}",
                    termion::cursor::Goto(1, self.height - 1),
                    line,
                )?;
            }
        }
        Ok(())
    }

    fn scroll_down(&mut self) -> Result<()> {
        if self.scroll_data.active {
            let output_range = self.height as i32 - 1;
            let max_start_index = self.history.inner.len() as i32 - output_range;
            let new_start_index = self.scroll_data.pos + 5;
            if new_start_index >= max_start_index as usize {
                self.reset_scroll()?;
            } else {
                self.scroll_data.pos = new_start_index;
                self.draw_scroll()?;
            }
        }
        Ok(())
    }

    fn scroll_lock(&mut self, _lock: bool) -> Result<()> {
        Ok(())
    }

    fn scroll_to(&mut self, _row: usize) -> Result<()> {
        Ok(())
    }

    fn scroll_top(&mut self) -> Result<()> {
        if self.history.inner.len() as u16 >= self.height - 1 {
            self.scroll_data.active = true;
            self.scroll_data.pos = 0;
            self.draw_scroll()?;
        }
        Ok(())
    }

    fn scroll_up(&mut self) -> Result<()> {
        let output_range = self.height as usize - 1;
        if self.history.inner.len() > output_range {
            if !self.scroll_data.active {
                self.scroll_data.active = true;
                self.scroll_data.pos = self.history.inner.len() - output_range;
            }
            self.scroll_data.pos -= self.scroll_data.pos.min(5);
            self.draw_scroll()?;
        }
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

    fn destroy(mut self: Box<Self>) -> Result<(Box<dyn Write>, super::history::History)> {
        self.reset()?;
        Ok((self.screen, self.history))
    }
}
