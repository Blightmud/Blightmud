use std::io::Write;

use anyhow::Result;
use termion::{
    clear,
    cursor::{self, Goto},
};

use crate::{
    model::{Line, Regex},
    ui::{
        printable_chars::PrintableCharsIterator, DisableOriginMode, ResetScrollRegion, ScrollRegion,
    },
};

use super::{
    history::History, scroll_data::ScrollData, user_interface::TerminalSizeError, wrap_line,
    UserInterface,
};

pub struct ReaderScreen {
    screen: Box<dyn Write>,
    history: History,
    scroll_data: ScrollData,
    output_line: u16,
    prompt_line: u16,
    width: u16,
    height: u16,
    prompt_input: Option<(String, usize)>,
}

impl ReaderScreen {
    pub fn new(screen: Box<dyn Write>, history: History) -> Result<Self> {
        let (width, height) = termion::terminal_size()?;
        let output_line = height - 1;
        let prompt_line = height;
        let scroll_data = ScrollData::new();
        Ok(Self {
            screen,
            history,
            scroll_data,
            output_line,
            prompt_line,
            width,
            height,
            prompt_input: None,
        })
    }

    #[inline]
    fn print(&mut self, line: &str, new_line: bool) {
        self.history.append(line);
        if !self.scroll_data.active {
            write!(
                self.screen,
                "{}{}{}{}",
                Goto(1, self.height - 1),
                if new_line { "\n" } else { "" },
                line,
                Goto(1, self.height)
            )
            .unwrap();
        }
    }

    #[inline]
    fn print_line(&mut self, line: &Line) {
        if let Some(print_line) = &line.print_line() {
            self.history.append(print_line);
            if !self.scroll_data.active {
                writeln!(
                    self.screen,
                    "{}\n{}{}",
                    Goto(1, self.height - 1),
                    print_line,
                    Goto(1, self.height)
                )
                .unwrap();
            }
        }
    }

    #[inline]
    fn print_wrapped_prompt_input(&mut self, line: &str, pos: usize) {
        let mut input = line;
        let mut pos = pos;
        let width = self.width as usize;
        while input.chars().count() >= width && pos >= width {
            if let Some((i, _)) = input.char_indices().nth(width) {
                input = input.split_at(i).1;
            } else {
                input = "";
            }
            pos -= width;
        }
        if input.chars().count() >= width {
            if let Some((i, _)) = input.char_indices().nth(width) {
                input = input.split_at(i).0;
            }
        }
        write!(
            self.screen,
            "{}{}{}{}",
            Goto(1, self.prompt_line),
            clear::CurrentLine,
            input,
            Goto(pos as u16 + 1, self.prompt_line)
        )
        .unwrap();
    }

    #[inline]
    fn print_prompt_input_suffix(&mut self, line: &str, start: usize, end: usize) {
        write!(
            self.screen,
            "{}{}{}",
            Goto(start as u16 + 1, self.prompt_line),
            line,
            Goto(end as u16 + 1, self.prompt_line)
        )
        .unwrap();
    }

    #[inline]
    fn trim_prompt_input(&mut self, pos: usize) {
        write!(
            self.screen,
            "{}{}",
            Goto(pos as u16 + 1, self.prompt_line),
            clear::AfterCursor,
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
                line,
                cursor::Goto(1, self.prompt_line),
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
            self.output_line = height - 1;
            self.prompt_line = height;
            self.width = width;
            self.height = height;
            write!(
                self.screen,
                "{}{}{}",
                ScrollRegion(1, self.output_line),
                DisableOriginMode,
                cursor::Goto(1, self.prompt_line),
            )?;
            self.reset_scroll()?;
            self.screen.flush()?;
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
            if let Some(print_line) = line.print_line() {
                self.history.remove_last_if_prefix(print_line);
            }
        }
        if let Some(print_line) = line.print_line() {
            if !line.is_utf8() || print_line.trim().is_empty() {
                self.print(print_line, !line.flags.separate_receives);
            } else {
                let mut new_line = !line.flags.separate_receives;
                let mut count = 0;
                let cur_line = self.history.len();
                for l in wrap_line(print_line, self.width as usize) {
                    self.print(l, new_line);
                    new_line = true;
                    count += 1;
                }
                if self.scroll_data.scroll_lock && count > self.output_line {
                    self.scroll_to(cur_line).ok();
                }
            }
        }
    }

    fn print_prompt(&mut self, prompt: &Line) {
        if !prompt.is_empty() {
            self.print_line(prompt);
        }
    }

    // This is fancy logic to make 'tdsr' less noisy
    fn print_prompt_input(&mut self, input: &str, pos: usize) {
        // Reader screens only operate on printable input characters (no term control sequences, e.g. ANSI colour).
        let sanitized_input = input.printable_chars().collect::<String>();
        let input = sanitized_input.as_str();
        let mut pos = pos;
        let width = self.width as usize;
        if let Some((existing, orig)) = &self.prompt_input {
            if (width - 1..width + 1).contains(&pos) {
                // Fall back to default behaviour when the prompt wraps
                self.print_wrapped_prompt_input(input, pos);
            } else {
                let mut orig = *orig;
                while pos >= width {
                    pos -= width;
                    if orig >= width {
                        orig -= width;
                    }
                }
                if input.starts_with(existing) {
                    let input = input[existing.len()..].to_owned();
                    self.print_prompt_input_suffix(&input, orig, pos);
                } else if existing.starts_with(input) {
                    self.trim_prompt_input(pos);
                } else {
                    self.print_wrapped_prompt_input(input, pos);
                }
            }
        } else {
            self.print_wrapped_prompt_input(input, pos);
        }
        self.prompt_input = Some((input.to_string(), pos));
    }

    fn print_send(&mut self, send: &Line) {
        if self.scroll_data.active && send.flags.source != Some("script".to_string()) {
            self.reset_scroll().ok();
        }
        if let Some(print_line) = send.print_line() {
            self.history.append(print_line);
        }
    }

    fn reset(&mut self) -> Result<()> {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion)?;
        Ok(())
    }

    fn reset_scroll(&mut self) -> Result<()> {
        self.scroll_data.reset(&self.history)?;
        let output_range = self.output_line;
        let output_start_index = self.history.inner.len() as i32 - output_range as i32;
        if output_start_index >= 0 {
            let output_start_index = output_start_index as usize;
            for i in 0..output_range {
                let index = output_start_index + i as usize;
                write!(
                    self.screen,
                    "{}{}{}{}",
                    cursor::Goto(1, 1 + i),
                    clear::AfterCursor,
                    self.history.inner[index],
                    cursor::Goto(1, self.prompt_line),
                )?;
            }
        } else {
            for line in &self.history.inner {
                write!(
                    self.screen,
                    "{}\n{}{}{}",
                    Goto(1, self.output_line),
                    clear::AfterCursor,
                    line,
                    cursor::Goto(1, self.prompt_line),
                )?;
            }
        }
        Ok(())
    }

    fn scroll_down(&mut self) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        if self.scroll_data.active {
            let output_range = self.output_line as i32;
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
        self.scroll_data.clamp(&self.history);
        if self.history.len() > self.output_line as usize {
            let max_start_index = self.history.inner.len() as i32 - self.output_line as i32;
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
        if self.history.inner.len() as u16 >= self.output_line {
            self.scroll_data.active = true;
            self.scroll_data.pos = 0;
            self.draw_scroll()?;
        }
        Ok(())
    }

    fn scroll_up(&mut self) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        let output_range = self.output_line as usize;
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
        self.scroll_data.clamp(&self.history);
        let scroll_range = self.output_line as usize;
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
        self.scroll_data.clamp(&self.history);
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

    fn add_tag(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn clear_tags(&mut self) -> Result<()> {
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
