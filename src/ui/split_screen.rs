use super::history::History;
use super::scroll_data::ScrollData;
use super::user_interface::TerminalSizeError;
use super::wrap_line;
use crate::io::SaveData;
use crate::model::{Settings, HIDE_TOPBAR};
use crate::{model::Line, model::Regex, ui::ansi::*};
use anyhow::Result;
use std::collections::HashSet;
use std::io::Write;
use termion::color::{self, Bg, Fg};
use termion::cursor;

use super::UserInterface;

const SCROLL_LIVE_BUFFER_SIZE: u16 = 10;

struct StatusArea {
    start_line: u16,
    width: u16,
    status_lines: Vec<Option<String>>,
    scroll_marker: bool,
}

impl StatusArea {
    fn new(height: u16, start_line: u16, width: u16) -> Self {
        let height = height.clamp(1, 5);
        Self {
            start_line,
            width,
            status_lines: vec![None; height as usize],
            scroll_marker: false,
        }
    }

    fn set_scroll_marker(&mut self, value: bool) {
        self.scroll_marker = value;
    }

    fn set_height(&mut self, height: u16, start_line: u16) {
        self.clear();
        self.status_lines.resize(height.clamp(1, 5) as usize, None);
        self.update_pos(start_line);
    }

    fn update_pos(&mut self, start_line: u16) {
        self.start_line = start_line;
    }

    fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    fn set_status_line(&mut self, index: usize, line: String) {
        let index = index.clamp(0, self.status_lines.len() - 1);
        if !line.trim().is_empty() {
            self.status_lines[index] = Some(line);
        } else {
            self.status_lines[index] = None;
        }
    }

    fn clear(&mut self) {
        self.status_lines = vec![None; self.status_lines.len()];
    }

    fn redraw_line(&mut self, screen: &mut impl Write, line_no: usize) -> Result<()> {
        let line_no = line_no.clamp(0, self.status_lines.len() - 1);
        let index = self.start_line as usize + line_no;

        let mut info = if self.scroll_marker && line_no == 0 {
            "(more) ".to_string()
        } else {
            String::new()
        };

        if let Some(Some(custom_info)) = self.status_lines.get(line_no as usize) {
            info = if info.is_empty() {
                custom_info.to_string()
            } else {
                format!("{}━ {} ", info, custom_info)
            };
        }

        if line_no == 0 || line_no == self.status_lines.len() - 1 {
            self.draw_bar(index, screen, &info)?;
        } else {
            self.draw_line(index, screen, &info)?;
        }

        Ok(())
    }

    fn redraw(&mut self, screen: &mut impl Write) -> Result<()> {
        for line in 0..self.status_lines.len() {
            self.redraw_line(screen, line)?;
        }
        Ok(())
    }

    fn draw_bar(&self, line: usize, screen: &mut impl Write, custom_info: &str) -> Result<()> {
        write!(
            screen,
            "{}{}{}",
            termion::cursor::Goto(1, line as u16),
            termion::clear::CurrentLine,
            Fg(color::Green),
        )?;

        let custom_info = if !custom_info.trim().is_empty() {
            format!(
                "━ {}{}{} ",
                custom_info.trim(),
                Fg(color::Reset),
                Fg(color::Green)
            )
        } else {
            "".to_string()
        };

        let info_line = Line::from(&custom_info);
        let stripped_chars = info_line.line().len() - info_line.clean_line().len();

        write!(
            screen,
            "{:━<1$}",
            &custom_info,
            self.width as usize + stripped_chars
        )?; // Print separator
        write!(screen, "{}", Fg(color::Reset))?;
        Ok(())
    }

    fn draw_line(&self, line: usize, screen: &mut impl Write, info: &str) -> Result<()> {
        write!(
            screen,
            "{}{}",
            termion::cursor::Goto(1, line as u16),
            termion::clear::CurrentLine,
        )?;

        write!(screen, "{}", info)?; // Print separator
        Ok(())
    }

    fn height(&self) -> u16 {
        self.status_lines.len() as u16
    }
}

pub struct SplitScreen {
    screen: Box<dyn Write>,
    width: u16,
    height: u16,
    output_start_line: u16,
    output_line: u16,
    prompt_line: u16,
    status_area: StatusArea,
    cursor_prompt_pos: u16,
    history: History,
    scroll_data: ScrollData,
    connection: Option<String>,
    tags: HashSet<String>,
    prompt_input: String,
    prompt_input_pos: usize,
}

impl UserInterface for SplitScreen {
    fn setup(&mut self) -> Result<()> {
        self.reset()?;

        let settings = Settings::try_load()?;

        // Get params in case screen resized
        let (width, height) = termion::terminal_size()?;
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            self.output_line = height - self.status_area.height() - 1;
            self.prompt_line = height;
            self.output_start_line = if settings.get(HIDE_TOPBAR)? { 1 } else { 2 };

            write!(
                self.screen,
                "{}{}",
                ScrollRegion(self.output_start_line, self.output_line),
                DisableOriginMode
            )
            .unwrap(); // Set scroll region, non origin mode
            self.redraw_top_bar()?;
            self.reset_scroll()?;
            self.redraw_status_area()?;
            self.screen.flush()?;
            write!(
                self.screen,
                "{}{}",
                termion::cursor::Goto(1, self.output_start_line),
                termion::cursor::Save
            )?;
            Ok(())
        } else {
            Err(TerminalSizeError.into())
        }
    }

    fn print_error(&mut self, output: &str) {
        let line = &format!("{}[!!] {}{}", Fg(color::Red), output, Fg(color::Reset));
        self.print_line(line, true);
    }

    fn print_info(&mut self, output: &str) {
        let line = &format!("[**] {}", output);
        self.print_line(line, true);
    }

    fn print_output(&mut self, line: &Line) {
        //debug!("UI: {:?}", line);
        if line.flags.separate_receives {
            if let Some(print_line) = line.print_line() {
                self.history.remove_last_if_prefix(print_line);
            }
        }
        if let Some(print_line) = line.print_line() {
            if !line.is_utf8() || print_line.trim().is_empty() {
                self.print_line(print_line, !line.flags.separate_receives);
            } else {
                let mut new_line = !line.flags.separate_receives;
                let mut count = 0;
                let cur_line = self.history.len();
                for l in wrap_line(print_line, self.width as usize) {
                    self.print_line(l, new_line);
                    new_line = true;
                    count += 1;
                }
                if self.scroll_data.scroll_lock && count > self.height {
                    self.scroll_to(cur_line).ok();
                }
            }
        }
    }

    fn print_prompt(&mut self, prompt: &Line) {
        //debug!("UI: {:?}", prompt);
        if let Some(prompt_line) = prompt.print_line() {
            if !prompt_line.is_empty() {
                self.history.append(prompt_line);
                if self.scroll_data.not_scrolled_or_split() {
                    write!(
                        self.screen,
                        "{}\n{}{}",
                        termion::cursor::Goto(1, self.output_line),
                        prompt_line,
                        self.goto_prompt(),
                    )
                    .unwrap();
                }
            }
        }
    }

    fn print_prompt_input(&mut self, input: &str, pos: usize) {
        // Sanity check
        debug_assert!(pos <= input.len());

        self.prompt_input = input.to_string();
        self.prompt_input_pos = pos;

        let mut input = input;
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
        self.cursor_prompt_pos = pos as u16 + 1;
        write!(
            self.screen,
            "{}{}{}{}{}{}{}{}{}",
            termion::cursor::Save,
            termion::cursor::Goto(1, self.prompt_line),
            Fg(termion::color::Reset),
            Bg(termion::color::Reset),
            termion::style::Reset,
            termion::clear::CurrentLine,
            input,
            termion::cursor::Restore,
            self.goto_prompt(),
        )
        .unwrap();
    }

    fn print_send(&mut self, send: &Line) {
        if self.scroll_data.active && send.flags.source != Some("script".to_string()) {
            self.reset_scroll().ok();
        }
        if let Some(line) = send.print_line() {
            let line = &format!(
                "{}{}> {}{}",
                termion::style::Reset,
                Fg(color::LightYellow),
                line,
                Fg(color::Reset),
            );
            for line in wrap_line(line, self.width as usize) {
                self.print_line(line, true);
            }
        }
    }

    fn reset(&mut self) -> Result<()> {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion)?;
        Ok(())
    }

    fn reset_scroll(&mut self) -> Result<()> {
        let reset_split = self.scroll_data.split;
        let reset_scroll = self.scroll_data.active;
        self.scroll_data.reset(&self.history)?;
        if reset_split {
            write!(self.screen, "{}", ResetScrollRegion)?;
            write!(
                self.screen,
                "{}{}",
                ScrollRegion(self.output_start_line, self.output_line),
                DisableOriginMode
            )?;
        } else if reset_scroll {
            self.status_area.set_scroll_marker(false);
            self.status_area.redraw_line(&mut self.screen, 0)?;
        }

        let output_range = self.output_range();
        let output_start_index = self.history.inner.len() as i32 - output_range as i32;
        if output_start_index >= 0 {
            let output_start_index = output_start_index as usize;
            for i in 0..output_range {
                let index = output_start_index + i as usize;
                let line_no = self.output_start_line + i;
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
                    termion::cursor::Goto(1, self.output_line),
                    line,
                )?;
            }
        }
        Ok(())
    }

    fn scroll_down(&mut self) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        if self.scroll_data.active {
            let output_range = self.scroll_range() as i32;
            let max_start_index: i32 = self.history.inner.len() as i32 - output_range;
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
        if self.history.len() > self.scroll_range() as usize {
            let max_start_index = self.history.inner.len() as i32 - self.scroll_range() as i32;
            if max_start_index > 0 && row < max_start_index as usize {
                self.init_scroll()?;
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
            self.init_scroll()?;
            self.scroll_data.pos = 0;
            self.draw_scroll()?;
        }
        Ok(())
    }

    fn scroll_up(&mut self) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        let output_range: usize = self.scroll_range() as usize;
        if self.history.inner.len() > output_range {
            if !self.scroll_data.active {
                self.init_scroll()?;
                self.scroll_data.pos = self.history.inner.len() - output_range;
            }
            self.scroll_data.pos -= self.scroll_data.pos.min(5);
            self.draw_scroll()?;
        }
        Ok(())
    }

    fn find_up(&mut self, pattern: &Regex) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        let pos = if self.scroll_data.active {
            self.scroll_data.pos
        } else if self.history.len() > self.scroll_range() as usize {
            self.history.len() - self.scroll_range() as usize
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

    fn set_host(&mut self, host: &str, port: u16) -> Result<()> {
        self.connection = if !host.is_empty() {
            Some(format!("{}:{}", host, port))
        } else {
            None
        };
        self.redraw_top_bar()
    }

    fn add_tag(&mut self, tag: &str) -> Result<()> {
        self.tags.insert(tag.to_string());
        self.redraw_top_bar()
    }

    fn clear_tags(&mut self) -> Result<()> {
        self.tags.clear();
        self.redraw_top_bar()
    }

    fn set_status_area_height(&mut self, height: u16) -> Result<()> {
        let height = height.clamp(1, 5);
        self.status_area
            .set_height(height, self.height - height - 1);
        self.setup()?;
        let input_str = self.prompt_input.as_str().to_owned();
        self.print_prompt_input(&input_str, self.prompt_input_pos);
        Ok(())
    }

    fn set_status_line(&mut self, line: usize, info: String) -> Result<()> {
        self.status_area.set_status_line(line, info);
        self.status_area.redraw_line(&mut self.screen, line)?;
        write!(self.screen, "{}", self.goto_prompt())?;
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

    fn destroy(mut self: Box<Self>) -> Result<(Box<dyn Write>, History)> {
        self.reset()?;
        Ok((self.screen, self.history))
    }
}

impl SplitScreen {
    pub fn new(screen: Box<dyn Write>, history: History) -> Result<Self> {
        let (width, height) = termion::terminal_size()?;

        let output_start_line = 2;
        let status_area_height = 1;
        let output_line = height - status_area_height - 1;
        let prompt_line = height;

        let status_area = StatusArea::new(status_area_height, output_line + 1, width);

        Ok(Self {
            screen,
            width,
            height,
            output_start_line,
            output_line,
            status_area,
            prompt_line,
            cursor_prompt_pos: 1,
            history,
            scroll_data: ScrollData::new(),
            connection: None,
            tags: HashSet::new(),
            prompt_input: String::new(),
            prompt_input_pos: 1,
        })
    }

    fn print_line(&mut self, line: &str, new_line: bool) {
        self.history.append(line);
        if self.scroll_data.not_scrolled_or_split() {
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, self.output_line),
                if new_line { "\r\n" } else { "" },
                &line,
                self.goto_prompt(),
            )
            .unwrap();
        }
    }

    fn redraw_top_bar(&mut self) -> Result<()> {
        if self.output_start_line > 1 {
            write!(
                self.screen,
                "{}{}{}",
                termion::cursor::Goto(1, 1),
                termion::clear::CurrentLine,
                Fg(color::Green),
            )?;
            let host = if let Some(connection) = &self.connection {
                format!("═ {} ", connection)
            } else {
                "".to_string()
            };
            let mut tags = self
                .tags
                .iter()
                .map(|s| format!("[{}]", s))
                .collect::<Vec<String>>();
            tags.sort();
            let tags = tags.join("");
            let mut output = format!("{}{}", host, tags);
            if !output.is_empty() {
                output.push(' ');
            }
            write!(self.screen, "{:═<1$}", output, self.width as usize)?; // Print separator
            write!(self.screen, "{}{}", Fg(color::Reset), self.goto_prompt(),)?;
        }
        Ok(())
    }

    fn redraw_status_area(&mut self) -> Result<()> {
        self.status_area.set_width(self.width);
        self.status_area.update_pos(self.output_line + 1);
        self.status_area.redraw(&mut self.screen)?;
        write!(self.screen, "{}", self.goto_prompt(),)?;
        Ok(())
    }

    fn goto_prompt(&self) -> String {
        format!(
            "{}",
            termion::cursor::Goto(self.cursor_prompt_pos, self.prompt_line),
        )
    }

    fn init_scroll(&mut self) -> Result<()> {
        self.scroll_data.active = true;
        if self.scroll_range() < self.output_range() {
            self.scroll_data.split = true;
            let scroll_range = self.scroll_range();
            write!(self.screen, "{}", ResetScrollRegion)?;
            write!(
                self.screen,
                "{}{}",
                ScrollRegion(scroll_range + 3, self.output_line),
                DisableOriginMode
            )?;
            write!(
                self.screen,
                "{}{}{:━<4$}{}",
                cursor::Goto(1, scroll_range + 2),
                color::Fg(color::Green),
                "━ (scroll) ",
                color::Fg(color::Reset),
                self.width as usize
            )?;
        } else {
            self.status_area.set_scroll_marker(true);
            self.status_area.redraw_line(&mut self.screen, 0)?;
        }
        Ok(())
    }

    fn draw_scroll(&mut self) -> Result<()> {
        let output_range = self.scroll_range();
        for i in 0..output_range {
            let index = self.scroll_data.pos + i as usize;
            let line_no = self.output_start_line + i;
            let mut line = self.history.inner[index].clone();
            if let Some(pattern) = &self.scroll_data.hilite {
                line = pattern
                    .replace_all(
                        &line,
                        format!(
                            "{}{}$0{}{}",
                            Fg(color::LightWhite),
                            Bg(color::Blue),
                            Bg(color::Reset),
                            Fg(color::Reset)
                        ),
                    )
                    .to_string();
            }
            write!(
                self.screen,
                "{}{}{}",
                termion::cursor::Goto(1, line_no),
                termion::clear::CurrentLine,
                line,
            )?;
        }
        Ok(())
    }

    fn scroll_range(&self) -> u16 {
        if self.scroll_data.allow_split && self.height > SCROLL_LIVE_BUFFER_SIZE * 2 {
            self.output_line - self.output_start_line - SCROLL_LIVE_BUFFER_SIZE + 1
        } else {
            self.output_range()
        }
    }

    fn output_range(&self) -> u16 {
        self.output_line - self.output_start_line + 1
    }
}

#[cfg(test)]
mod screen_test {
    use super::*;

    #[test]
    fn test_append_history() {
        let line = "a nice line\n\nwith a blank line\nand lines\nc\ntest\n";

        let mut history = History::new();
        history.append(line);
        assert_eq!(
            history.inner,
            vec![
                "a nice line",
                "",
                "with a blank line",
                "and lines",
                "c",
                "test",
            ]
        );
    }

    #[test]
    fn test_search_history() {
        let line = "a nice line\n\nwith a blank line\nand lines\nc\ntest\n";

        let mut history = History::new();
        history.append(line);
        let re = crate::model::Regex::new("and lines", None).unwrap();
        assert_eq!(history.find_forward(&re, 0), Some(3));
        assert_eq!(history.find_forward(&re, 4), None);
        assert_eq!(history.find_backward(&re, 4), Some(3));
        assert_eq!(history.find_backward(&re, 2), None);
    }

    #[test]
    fn test_drain_history() {
        let mut history = History::new();
        history.capacity = 20;
        history.drain_length = 10;
        assert!(history.is_empty());
        for _ in 0..19 {
            history.append("test");
        }
        assert_eq!(history.len(), 19);
        history.append("test");
        assert_eq!(history.len(), 10);
        for _ in 0..9 {
            history.append("test");
        }
        assert_eq!(history.len(), 19);
        history.append("test");
        assert_eq!(history.len(), 10);
    }
}
