use crate::{model::Line, ui::ansi::*};
use anyhow::Result;
use std::collections::VecDeque;
use std::io::{stdout, Stdout, Write};
use std::{error, fmt};
use termion::{
    color,
    input::MouseTerminal,
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
};

struct ScrollData(bool, usize);
const OUTPUT_START_LINE: u16 = 2;

#[derive(Debug)]
struct TerminalSizeError;

impl fmt::Display for TerminalSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to retrieve valid dimsensions for terminal")
    }
}

impl error::Error for TerminalSizeError {
    fn description(&self) -> &str {
        "Failed to retrieve valid dimensions for terminal"
    }
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

struct StatusArea {
    start_line: u16,
    end_line: u16,
    width: u16,
    status_lines: Vec<Option<String>>,
}

type ScreenHandle = MouseTerminal<AlternateScreen<RawTerminal<Stdout>>>;

impl StatusArea {
    fn new(height: u16, start_line: u16, width: u16) -> Self {
        let height = height.min(5).max(1);
        let end_line = start_line + height - 1;
        Self {
            start_line,
            end_line,
            width,
            status_lines: vec![None; height as usize],
        }
    }

    fn set_height(&mut self, height: u16, start_line: u16) {
        self.clear();
        self.status_lines
            .resize(height.min(5).max(1) as usize, None);
        self.update_pos(start_line);
    }

    fn update_pos(&mut self, start_line: u16) {
        self.start_line = start_line;
    }

    fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    fn set_status_line(&mut self, index: usize, line: String) {
        let index = index.max(0).min(self.status_lines.len() - 1);
        if !line.trim().is_empty() {
            self.status_lines[index] = Some(line);
        } else {
            self.status_lines[index] = None;
        }
    }

    fn clear(&mut self) {
        self.status_lines = vec![None; self.status_lines.len()];
    }

    fn redraw(&mut self, screen: &mut ScreenHandle, scrolled: bool) -> Result<()> {
        for line in self.start_line..self.end_line + 1 {
            write!(
                screen,
                "{}{}",
                termion::cursor::Goto(1, line),
                termion::clear::CurrentLine,
            )?;
        }

        let mut info = if scrolled {
            "(more) ".to_string()
        } else {
            "".to_string()
        };

        if let Some(Some(custom_info)) = self.status_lines.get(0) {
            if info.is_empty() {
                info = custom_info.to_string();
            } else {
                info = format!("{}━ {} ", info, custom_info);
            }
        }

        self.draw_bar(self.start_line, screen, &info)?;
        if self.start_line != self.end_line {
            let height = self.status_lines.len() as u16;
            for line_no in 1..height {
                let line_no = line_no as u16;
                let info = if let Some(info) = &self.status_lines[line_no as usize] {
                    &info
                } else {
                    ""
                };

                if line_no == height - 1 {
                    self.draw_bar(self.start_line + line_no, screen, &info)?;
                } else {
                    self.draw_line(self.start_line + line_no, screen, &info)?;
                }
            }
        }
        Ok(())
    }

    fn draw_bar(&self, line: u16, screen: &mut ScreenHandle, custom_info: &str) -> Result<()> {
        write!(
            screen,
            "{}{}{}",
            termion::cursor::Goto(1, line),
            termion::clear::CurrentLine,
            color::Fg(color::Green),
        )?;

        let custom_info = if !custom_info.trim().is_empty() {
            format!(
                "━ {}{}{} ",
                custom_info.trim(),
                color::Fg(color::Reset),
                color::Fg(color::Green)
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
        write!(screen, "{}", color::Fg(color::Reset))?;
        Ok(())
    }

    fn draw_line(&self, line: u16, screen: &mut ScreenHandle, info: &str) -> Result<()> {
        write!(
            screen,
            "{}{}",
            termion::cursor::Goto(1, line),
            termion::clear::CurrentLine,
        )?;

        write!(screen, "{}", info)?; // Print separator
        Ok(())
    }

    fn height(&self) -> u16 {
        self.status_lines.len() as u16
    }
}

struct History {
    inner: VecDeque<String>,
}

impl History {
    fn new() -> Self {
        Self {
            inner: VecDeque::with_capacity(32 * 1024),
        }
    }

    fn append(&mut self, line: &str) {
        if !line.trim().is_empty() {
            for line in line.lines() {
                self.inner.push_back(String::from(line));
            }
        } else {
            self.inner.push_back("".to_string());
        }
        while self.inner.len() >= self.inner.capacity() {
            self.inner.pop_front();
        }
    }
}

pub struct Screen {
    screen: ScreenHandle,
    pub width: u16,
    pub height: u16,
    output_line: u16,
    prompt_line: u16,
    status_area: StatusArea,
    cursor_prompt_pos: u16,
    history: History,
    scroll_data: ScrollData,
    connection: Option<String>,
}

impl Screen {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        let screen = MouseTerminal::from(AlternateScreen::from(stdout().into_raw_mode()?));
        let (width, height) = termion::terminal_size()?;

        let status_area_height = 1;
        let output_line = height - status_area_height - 1;
        let prompt_line = height;

        let status_area = StatusArea::new(status_area_height, output_line + 1, width);

        Ok(Self {
            screen,
            width,
            height,
            output_line,
            status_area,
            prompt_line,
            cursor_prompt_pos: 1,
            history: History::new(),
            scroll_data: ScrollData(false, 0),
            connection: None,
        })
    }

    pub fn setup(&mut self) -> Result<()> {
        self.reset()?;
        write!(self.screen, "{}", termion::clear::All)?;

        // Get params in case screen resized
        let (width, height) = termion::terminal_size()?;
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            self.output_line = height - self.status_area.height() - 1;
            self.prompt_line = height;

            write!(
                self.screen,
                "{}{}",
                ScrollRegion(OUTPUT_START_LINE, self.output_line),
                DisableOriginMode
            )
            .unwrap(); // Set scroll region, non origin mode
            self.redraw_top_bar()?;
            self.reset_scroll()?;
            self.screen.flush()?;
            write!(
                self.screen,
                "{}{}",
                termion::cursor::Goto(1, 2),
                termion::cursor::Save
            )?;
            Ok(())
        } else {
            Err(TerminalSizeError.into())
        }
    }

    pub fn set_status_area_height(&mut self, height: u16) -> Result<()> {
        let height = height.max(1).min(5);
        self.status_area.set_height(height, self.height);
        self.setup()?;
        Ok(())
    }

    pub fn set_status_line(&mut self, line: usize, info: String) -> Result<()> {
        self.status_area.set_status_line(line, info);
        self.status_area
            .redraw(&mut self.screen, self.scroll_data.0)?;
        write!(self.screen, "{}", self.goto_prompt())?;
        Ok(())
    }

    pub fn set_host(&mut self, host: &str, port: u16) -> Result<()> {
        self.connection = if !host.is_empty() {
            Some(format!("{}:{}", host, port))
        } else {
            None
        };
        self.redraw_top_bar()
    }

    fn redraw_top_bar(&mut self) -> Result<()> {
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::CurrentLine,
            color::Fg(color::Green),
        )?;
        let output = if let Some(connection) = &self.connection {
            format!("═ {} ═", connection)
        } else {
            "".to_string()
        };
        write!(self.screen, "{:═<1$}", output, self.width as usize)?; // Print separator
        write!(
            self.screen,
            "{}{}",
            color::Fg(color::Reset),
            self.goto_prompt(),
        )?;
        Ok(())
    }

    fn redraw_status_area(&mut self) -> Result<()> {
        self.status_area.set_width(self.width);
        self.status_area.update_pos(self.output_line + 1);
        self.status_area
            .redraw(&mut self.screen, self.scroll_data.0)?;
        write!(self.screen, "{}", self.goto_prompt(),)?;
        Ok(())
    }

    fn goto_prompt(&self) -> String {
        format!(
            "{}",
            termion::cursor::Goto(self.cursor_prompt_pos, self.prompt_line),
        )
    }

    pub fn reset(&mut self) -> Result<()> {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion)?;
        Ok(())
    }

    pub fn print_prompt(&mut self, prompt: &Line) {
        if let Some(prompt_line) = prompt.print_line() {
            self.history.append(prompt_line);
            if !self.scroll_data.0 {
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

    pub fn print_prompt_input(&mut self, input: &str, pos: usize) {
        // Sanity check
        debug_assert!(pos <= input.len());

        let mut input = input;
        let mut pos = pos;
        let width = self.width as usize;
        while input.len() >= width && pos >= width {
            let (_, last) = input.split_at(self.width as usize);
            input = last;
            pos -= width;
        }
        if input.len() >= width {
            input = input.split_at(width).0;
        }
        self.cursor_prompt_pos = pos as u16 + 1;
        write!(
            self.screen,
            "{}{}{}{}{}{}{}{}{}",
            termion::cursor::Save,
            termion::cursor::Goto(1, self.prompt_line),
            termion::color::Fg(termion::color::Reset),
            termion::color::Bg(termion::color::Reset),
            termion::style::Reset,
            termion::clear::CurrentLine,
            input,
            termion::cursor::Restore,
            self.goto_prompt(),
        )
        .unwrap();
    }

    pub fn print_output(&mut self, line: &Line) {
        if let Some(print_line) = line.print_line() {
            if !line.is_utf8() || print_line.trim().is_empty() {
                self.print_line(&print_line);
            } else {
                for line in wrap_line(&print_line, self.width as usize) {
                    self.print_line(&line);
                }
            }
        }
    }

    fn print_line(&mut self, line: &str) {
        self.history.append(&line);
        if !self.scroll_data.0 {
            write!(
                self.screen,
                "{}\n{}{}",
                termion::cursor::Goto(1, self.output_line),
                &line,
                self.goto_prompt(),
            )
            .unwrap();
        }
    }

    pub fn print_send(&mut self, send: &Line) {
        if let Some(line) = send.print_line() {
            self.print_line(&format!(
                "{}> {}{}",
                color::Fg(color::LightYellow),
                line,
                color::Fg(color::Reset)
            ));
        }
    }

    pub fn print_info(&mut self, output: &str) {
        self.print_line(&format!("[**] {}", output));
    }

    pub fn print_error(&mut self, output: &str) {
        self.print_line(&format!(
            "{}[!!] {}{}",
            color::Fg(color::Red),
            output,
            color::Fg(color::Reset)
        ));
    }

    pub fn scroll_up(&mut self) -> Result<()> {
        let output_range: usize = self.output_line as usize - OUTPUT_START_LINE as usize;
        let history = &self.history.inner;
        if history.len() > output_range as usize {
            if !self.scroll_data.0 {
                self.scroll_data.0 = true;
                self.scroll_data.1 = history.len() - output_range;
            }
            self.scroll_data.0 = true;
            self.scroll_data.1 -= self.scroll_data.1.min(5);
            self.draw_scroll()?;
        }
        Ok(())
    }

    pub fn scroll_down(&mut self) -> Result<()> {
        if self.scroll_data.0 {
            let output_range: i32 = self.output_line as i32 - OUTPUT_START_LINE as i32;
            let max_start_index: i32 = self.history.inner.len() as i32 - output_range;
            let new_start_index = self.scroll_data.1 + 5;
            if new_start_index >= max_start_index as usize {
                self.reset_scroll()?;
            } else {
                self.scroll_data.1 = new_start_index;
                self.draw_scroll()?;
            }
        }
        Ok(())
    }

    fn draw_scroll(&mut self) -> Result<()> {
        let output_range = self.output_line - OUTPUT_START_LINE + 1;
        for i in 0..output_range {
            let index = self.scroll_data.1 + i as usize;
            let line_no = OUTPUT_START_LINE + i;
            write!(
                self.screen,
                "{}{}{}",
                termion::cursor::Goto(1, line_no),
                termion::clear::CurrentLine,
                self.history.inner[index],
            )?;
        }
        self.redraw_status_area()?;
        Ok(())
    }

    pub fn reset_scroll(&mut self) -> Result<()> {
        self.scroll_data.0 = false;
        let output_range = self.output_line - OUTPUT_START_LINE + 1;
        let output_start_index = self.history.inner.len() as i32 - output_range as i32;
        if output_start_index >= 0 {
            let output_start_index = output_start_index as usize;
            for i in 0..output_range {
                let index = output_start_index + i as usize;
                let line_no = OUTPUT_START_LINE + i;
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
        self.redraw_status_area()?;
        Ok(())
    }

    pub fn flush(&mut self) {
        self.screen.flush().unwrap();
    }
}

fn wrap_line(line: &str, width: usize) -> Vec<&str> {
    let mut lines: Vec<&str> = vec![];

    for line in line.lines() {
        // If the line is empty just push and continue
        if line.trim().is_empty() {
            lines.push(&line);
            continue;
        }

        let mut last_cut: usize = 0;
        let mut last_space: usize = 0;
        let mut print_length = 0;
        let mut print_length_since_space = 0;
        let mut in_escape = false;
        for (length, c) in line.chars().enumerate() {
            // Check for escape sequences
            if c == '\x1b' {
                in_escape = true;
                continue;
            }

            // Check for escape sequence endings
            if in_escape {
                in_escape = c != 'm';
                continue;
            }

            // Keep track of printable line length
            print_length += 1;

            // Keep track of last occurence of <space> and how many printable
            // characters followed it
            print_length_since_space += 1;
            if c == ' ' && print_length < width {
                last_space = length;
                print_length_since_space = 0;
            }

            // Split the line if it's print length reaches screen width
            if print_length >= width {
                // Cut from last space if there is any. Otherwise just cut.
                if last_cut < last_space {
                    lines.push(&line[last_cut..last_space]);
                    print_length = print_length_since_space;
                    last_cut = last_space + 1;
                } else {
                    lines.push(&line[last_cut..length + 1]);
                    print_length = 0;
                    last_cut = length + 1;
                }
            }
        }

        // Push the rest of the line if there is anything left
        if last_cut < line.len() && !line[last_cut..].trim().is_empty() {
            lines.push(&line[last_cut..]);
        }
    }
    lines
}

#[cfg(test)]
mod screen_test {
    use super::*;

    #[test]
    fn test_wrap_line() {
        let line: &'static str =
            "\x1b[34mSomething \x1b[0mthat's pretty \x1b[32mlong and annoying\x1b[0m";
        let lines = wrap_line(&line, 11);
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&"\u{1b}[34mSomething"));
        assert_eq!(iter.next(), Some(&"\u{1b}[0mthat's"));
        assert_eq!(iter.next(), Some(&"pretty"));
        assert_eq!(iter.next(), Some(&"\u{1b}[32mlong and"));
        assert_eq!(iter.next(), Some(&"annoying\u{1b}[0m"));
    }

    #[test]
    fn test_long_line_no_space() {
        let mut line = String::new();
        for _ in 0..1000 {
            for i in 0..10 {
                let num = format!("{}", i);
                line = format!(
                    "{}{}",
                    line,
                    std::iter::repeat(num).take(15).collect::<String>()
                );
            }
        }
        let lines = wrap_line(&line, 15);
        assert_eq!(lines.len(), 1000 * 10);
        for (i, line) in lines.iter().enumerate() {
            let num = format!("{}", i % 10);
            assert_eq!(
                line,
                &format!("{}", std::iter::repeat(num).take(15).collect::<String>())
            );
        }
    }

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
}
