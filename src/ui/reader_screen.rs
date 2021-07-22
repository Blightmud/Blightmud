use std::io::Write;

use anyhow::Result;
use termion::{
    clear,
    cursor::{self, Goto},
};

use crate::model::{Line, Regex};

use super::{
    history::History, scroll_data::ScrollData, user_interface::TerminalSizeError, wrap_line,
    UserInterface,
};

pub struct ReaderScreen {
    screen: Box<dyn Write>,
    history: History,
    scroll_data: ScrollData,
    pub width: u16,
    pub height: u16,
    prompt_input: Option<(String, usize)>,
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
            prompt_input: None,
        })
    }

    fn print(&mut self, line: &str, new_line: bool) {
        self.history.append(line);
        if !self.scroll_data.active {
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
            self.print_typed_prompt();
        }
    }

    fn print_line(&mut self, line: &Line) {
        self.history.append(&line.to_string());
        if !self.scroll_data.active {
            writeln!(
                self.screen,
                "{}{}{}{}",
                Goto(1, self.height),
                clear::AfterCursor,
                line,
                Goto(1, self.height)
            )
            .unwrap();
            self.print_typed_prompt();
        }
    }

    fn print_typed_prompt(&mut self) {
        if let Some((line, pos)) = &self.prompt_input {
            write!(
                self.screen,
                "{}{}{}{}",
                Goto(1, self.height),
                clear::AfterCursor,
                line,
                Goto(*pos as u16 + 1, self.height)
            )
            .unwrap();
        }
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
        let (width, height) = termion::terminal_size()?;
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            self.reset_scroll()?;
            self.screen.flush()?;
            write!(self.screen, "{}", termion::cursor::Goto(1, height),)?;
            Ok(())
        } else {
            Err(TerminalSizeError.into())
        }
    }

    fn print_error(&mut self, output: &str) {
        self.print_line(&Line::from(format!("ERROR: {}", output)));
    }

    fn print_info(&mut self, output: &str) {
        self.print_line(&Line::from(format!("INFO: {}", output)));
    }

    fn print_output(&mut self, line: &Line) {
        if line.flags.separate_receives {
            if let Some(prefix) = self.history.remove_last() {
                debug_assert!(line.print_line().unwrap().starts_with(&prefix));
            }
        }
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
        self.prompt_input = Some((input.to_string(), pos));
        self.print_typed_prompt();
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
            write!(
                self.screen,
                "{}",
                cursor::Goto(1, output_start_index as u16)
            )?;
            for i in 0..output_range {
                let index = output_start_index + i as usize;
                writeln!(
                    self.screen,
                    "{}{}{}",
                    clear::AfterCursor,
                    self.history.inner[index],
                    cursor::Goto(1, self.height)
                )?;
            }
        } else {
            for line in &self.history.inner {
                writeln!(
                    self.screen,
                    "{}{}{}",
                    Goto(1, self.height),
                    clear::AfterCursor,
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

    fn scroll_lock(&mut self, lock: bool) -> Result<()> {
        self.scroll_data.lock(lock)
    }

    fn scroll_to(&mut self, row: usize) -> Result<()> {
        if self.history.len() > self.height as usize - 1 {
            let max_start_index = self.history.inner.len() as i32 - self.height as i32 - 1;
            if max_start_index > 0 && row < max_start_index as usize {
                self.scroll_data.active = true;
                self.scroll_data.pos = row;
                self.draw_scroll()?;
            } else {
                self.reset_scroll()?;
            }
        }
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

    fn find_up(&mut self, pattern: &Regex) -> Result<()> {
        let scroll_range = self.height as usize - 1;
        let pos = if self.scroll_data.active {
            self.scroll_data.pos
        } else if self.history.len() > scroll_range {
            self.history.len() - scroll_range
        } else {
            self.history.len()
        };
        if let Some(line) = self.history.find_backward(pattern, pos) {
            self.scroll_data.hilite = Some(pattern.clone());
            self.scroll_to(0.max(line) as usize)?;
        }
        Ok(())
    }

    fn find_down(&mut self, pattern: &Regex) -> Result<()> {
        if self.scroll_data.active {
            if let Some(line) = self
                .history
                .find_forward(pattern, self.history.len().min(self.scroll_data.pos + 1))
            {
                self.scroll_data.hilite = Some(pattern.clone());
                self.scroll_to(line.min(self.history.len() - 1))?;
            }
        }
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
