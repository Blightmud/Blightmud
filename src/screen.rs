use crate::{ansi::*, BlightResult};
use std::collections::VecDeque;
use std::io::{stdout, Stdout, Write};
use std::{error, fmt};
use termion::{
    color,
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

pub struct Screen {
    screen: AlternateScreen<RawTerminal<Stdout>>,
    width: u16,
    _height: u16,
    output_line: u16,
    prompt_line: u16,
    cursor_prompt_pos: u16,
    history: VecDeque<String>,
    scroll_data: ScrollData,
}

impl Screen {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        let screen = AlternateScreen::from(stdout().into_raw_mode()?);
        let (width, height) = termion::terminal_size()?;

        let output_line = height - 2;
        let prompt_line = height;

        Ok(Self {
            screen,
            width,
            _height: height,
            output_line,
            prompt_line,
            cursor_prompt_pos: 1,
            history: VecDeque::with_capacity(1024),
            scroll_data: ScrollData(false, 0),
        })
    }

    pub fn setup(&mut self) -> BlightResult {
        self.reset()?;

        // Get params in case screen resized
        let (width, height) = termion::terminal_size()?;
        if width > 0 && height > 0 {
            self.width = width;
            self._height = height;
            self.output_line = height - 2;
            self.prompt_line = height;

            write!(
                self.screen,
                "{}{}",
                ScrollRegion(OUTPUT_START_LINE, self.output_line),
                DisableOriginMode
            )
            .unwrap(); // Set scroll region, non origin mode
            self.redraw_top_bar("", 0)?;
            self.redraw_bottom_bar()?;
            self.screen.flush()?;
            Ok(())
        } else {
            Err(TerminalSizeError.into())
        }
    }

    pub fn redraw_top_bar(&mut self, host: &str, port: u16) -> BlightResult {
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::CurrentLine,
            color::Fg(color::Green),
        )?;
        let output = if !host.is_empty() {
            format!("═ {}:{} ═", host, port)
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

    fn redraw_bottom_bar(&mut self) -> BlightResult {
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, self.output_line + 1),
            termion::clear::CurrentLine,
            color::Fg(color::Green),
        )?;

        let info = if self.scroll_data.0 {
            "━ (more) ".to_string()
        } else {
            "".to_string()
        };

        write!(self.screen, "{:━<1$}", info, self.width as usize)?; // Print separator
        write!(
            self.screen,
            "{}{}",
            color::Fg(color::Reset),
            self.goto_prompt(),
        )?;
        Ok(())
    }

    fn goto_prompt(&self) -> termion::cursor::Goto {
        termion::cursor::Goto(self.cursor_prompt_pos, self.prompt_line)
    }

    pub fn reset(&mut self) -> BlightResult {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion)?;
        Ok(())
    }

    pub fn print_prompt(&mut self, prompt: &str) {
        self.append_to_history(prompt);
        if !self.scroll_data.0 {
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, self.output_line),
                termion::scroll::Up(1),
                prompt.trim_end(),
                self.goto_prompt(),
            )
            .unwrap();
        }
    }

    pub fn print_prompt_input(&mut self, input: &str, pos: usize) {
        let mut input = input;
        while input.len() >= self.width as usize {
            let (_, last) = input.split_at(self.width as usize);
            input = last;
        }
        self.cursor_prompt_pos = pos as u16 + 1;
        write!(
            self.screen,
            "{}{}{}{}",
            termion::cursor::Goto(1, self.prompt_line),
            termion::clear::CurrentLine,
            input,
            self.goto_prompt(),
        )
        .unwrap();
    }

    pub fn print_output(&mut self, line: &str) {
        if line.trim().is_empty() {
            self.print_line(&line);
        } else {
            for line in wrap_line(&line, self.width as usize) {
                self.print_line(&line);
            }
        }
    }

    fn print_line(&mut self, line: &str) {
        self.append_to_history(&line);
        if !self.scroll_data.0 {
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, self.output_line),
                termion::scroll::Up(1),
                &line,
                self.goto_prompt(),
            )
            .unwrap();
        }
    }

    pub fn print_send(&mut self, send: &str) {
        self.print_output(&format!(
            "{}> {}{}",
            color::Fg(color::LightYellow),
            send,
            color::Fg(color::Reset)
        ));
    }

    pub fn print_info(&mut self, output: &str) {
        self.print_output(&format!("[**] {}", output));
    }

    pub fn print_error(&mut self, output: &str) {
        self.print_output(&format!(
            "{}[!!] {}{}",
            color::Fg(color::Red),
            output,
            color::Fg(color::Reset)
        ));
    }

    pub fn scroll_up(&mut self) -> BlightResult {
        let output_range: usize = self.output_line as usize - OUTPUT_START_LINE as usize;
        if self.history.len() > output_range as usize {
            if !self.scroll_data.0 {
                self.scroll_data.0 = true;
                self.scroll_data.1 = self.history.len() - output_range;
            }
            self.scroll_data.0 = true;
            self.scroll_data.1 -= self.scroll_data.1.min(5);
            self.draw_scroll()?;
        }
        Ok(())
    }

    pub fn scroll_down(&mut self) -> BlightResult {
        if self.scroll_data.0 {
            let output_range: i32 = self.output_line as i32 - OUTPUT_START_LINE as i32;
            let max_start_index: i32 = self.history.len() as i32 - output_range;
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

    fn draw_scroll(&mut self) -> BlightResult {
        let output_range = self.output_line - OUTPUT_START_LINE + 1;
        for i in 0..output_range {
            let index = self.scroll_data.1 + i as usize;
            let line_no = OUTPUT_START_LINE + i;
            write!(
                self.screen,
                "{}{}{}",
                termion::cursor::Goto(1, line_no),
                termion::clear::CurrentLine,
                self.history[index],
            )?;
        }
        self.redraw_bottom_bar()?;
        Ok(())
    }

    pub fn reset_scroll(&mut self) -> BlightResult {
        self.scroll_data.0 = false;
        let output_range = self.output_line - OUTPUT_START_LINE + 1;
        let output_start_index = self.history.len() as i32 - output_range as i32;
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
                    self.history[index],
                )?;
            }
        } else {
            for line in &self.history {
                write!(
                    self.screen,
                    "{}{}{}",
                    termion::cursor::Goto(1, self.output_line),
                    termion::scroll::Up(1),
                    line,
                )?;
            }
        }
        self.redraw_bottom_bar()?;
        Ok(())
    }

    pub fn flush(&mut self) {
        self.screen.flush().unwrap();
    }

    fn append_to_history(&mut self, line: &str) {
        let lines = line.split("\r\n");
        for line in lines {
            self.history.push_back(String::from(line));
        }
        while self.history.len() >= self.history.capacity() {
            self.history.pop_front();
        }
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
        if last_cut + 1 < line.len() {
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
}
