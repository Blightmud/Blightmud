use crate::io::SaveData;
use crate::model::{Regex, Settings, SCROLL_LOCK, SCROLL_SPLIT};
use crate::{model::Line, tts::TTSController, ui::ansi::*};
use anyhow::Result;
use log::debug;
use std::{error, fmt};
use std::{
    io::{stdout, Write},
    sync::Arc,
    sync::Mutex,
};
use termion::cursor;
use termion::{
    color::{self, Bg, Fg},
    input::MouseTerminal,
    raw::IntoRawMode,
    screen::AlternateScreen,
};

#[cfg(test)]
use mockall::automock;

struct ScrollData {
    active: bool,
    split: bool,
    pos: usize,
    scroll_lock: bool,
    hilite: Option<Regex>,
    allow_split: bool,
    allow_scroll_lock: bool,
}

impl ScrollData {
    fn new() -> Self {
        let settings = Settings::load();
        Self {
            active: false,
            split: false,
            pos: 0,
            scroll_lock: false,
            hilite: None,
            allow_split: settings.get(SCROLL_SPLIT).unwrap_or(true),
            allow_scroll_lock: settings.get(SCROLL_LOCK).unwrap_or(true),
        }
    }

    fn reset(&mut self, history: &History) -> Result<()> {
        self.active = false;
        self.split = false;
        self.hilite = None;
        self.pos = if history.is_empty() {
            0
        } else {
            history.len() - 1
        };
        let settings = Settings::try_load()?;
        self.allow_split = settings.get(SCROLL_SPLIT).unwrap_or(true);
        self.allow_scroll_lock = settings.get(SCROLL_LOCK).unwrap_or(true);
        Ok(())
    }

    fn lock(&mut self, lock: bool) -> Result<()> {
        self.scroll_lock = lock && self.allow_scroll_lock;
        Ok(())
    }

    fn not_scrolled_or_split(&self) -> bool {
        !self.active || self.split
    }
}

const OUTPUT_START_LINE: u16 = 2;
const SCROLL_LIVE_BUFFER_SIZE: u16 = 10;

#[derive(Debug)]
struct TerminalSizeError;

impl fmt::Display for TerminalSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to retrieve valid dimensions for terminal")
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
    width: u16,
    status_lines: Vec<Option<String>>,
    scroll_marker: bool,
}

#[cfg_attr(test, automock)]
pub trait UserInterface {
    fn print_error(&mut self, output: &str);
    fn print_info(&mut self, output: &str);
    fn print_output(&mut self, line: &Line);
    fn print_prompt(&mut self, prompt: &Line);
    fn print_prompt_input(&mut self, input: &str, pos: usize);
    fn print_send(&mut self, send: &Line);
    fn reset(&mut self) -> Result<()>;
    fn reset_scroll(&mut self) -> Result<()>;
    fn scroll_down(&mut self) -> Result<()>;
    fn scroll_lock(&mut self, lock: bool) -> Result<()>;
    fn scroll_to(&mut self, row: usize) -> Result<()>;
    fn scroll_top(&mut self) -> Result<()>;
    fn scroll_up(&mut self) -> Result<()>;
    fn find_up(&mut self, pattern: &Regex) -> Result<()>;
    fn find_down(&mut self, pattern: &Regex) -> Result<()>;
    fn set_host(&mut self, host: &str, port: u16) -> Result<()>;
    fn set_status_area_height(&mut self, height: u16) -> Result<()>;
    fn set_status_line(&mut self, line: usize, info: String) -> Result<()>;
    fn flush(&mut self);
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

struct History {
    inner: Vec<String>,
    capacity: usize,
    drain_length: usize,
}

impl History {
    fn new() -> Self {
        let drain_length: usize = 1024;
        let capacity: usize = 32 * drain_length;
        Self {
            inner: Vec::with_capacity(capacity),
            capacity,
            drain_length,
        }
    }

    fn drain(&mut self) {
        if self.inner.len() >= self.capacity {
            self.inner.drain(0..self.drain_length);
        }
    }

    fn append(&mut self, line: &str) {
        if !line.trim().is_empty() {
            for line in line.lines() {
                self.inner.push(String::from(line));
            }
        } else {
            self.inner.push("".to_string());
        }
        self.drain();
    }

    fn remove_last(&mut self) -> Option<String> {
        self.inner.pop()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn find_forward(&self, pattern: &Regex, pos: usize) -> Option<usize> {
        self.inner[pos..]
            .iter()
            .position(|l| pattern.is_match(l))
            .map(|index| pos + index)
    }

    fn find_backward(&self, pattern: &Regex, pos: usize) -> Option<usize> {
        self.inner[..pos]
            .iter()
            .rev()
            .position(|l| pattern.is_match(l))
            .map(|index| pos - index - 1)
    }
}

pub struct Screen {
    screen: Box<dyn Write>,
    tts_ctrl: Arc<Mutex<TTSController>>,
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

impl UserInterface for Screen {
    fn set_status_area_height(&mut self, height: u16) -> Result<()> {
        let height = height.clamp(1, 5);
        self.status_area
            .set_height(height, self.height - height - 1);
        self.setup()?;
        Ok(())
    }

    fn set_status_line(&mut self, line: usize, info: String) -> Result<()> {
        self.status_area.set_status_line(line, info);
        self.status_area.redraw_line(&mut self.screen, line)?;
        write!(self.screen, "{}", self.goto_prompt())?;
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

    fn reset(&mut self) -> Result<()> {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion)?;
        Ok(())
    }

    fn print_prompt(&mut self, prompt: &Line) {
        debug!("UI: {:?}", prompt);
        self.tts_ctrl.lock().unwrap().speak_line(prompt);
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

    fn print_output(&mut self, line: &Line) {
        debug!("UI: {:?}", line);
        self.tts_ctrl.lock().unwrap().speak_line(line);
        if line.flags.separate_receives {
            if let Some(prefix) = self.history.remove_last() {
                debug_assert!(line.print_line().unwrap().starts_with(&prefix));
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

    fn print_send(&mut self, send: &Line) {
        if let Some(line) = send.print_line() {
            self.tts_ctrl.lock().unwrap().speak_input(line);
            self.print_line(
                &format!(
                    "{}{}> {}{}",
                    termion::style::Reset,
                    Fg(color::LightYellow),
                    line,
                    Fg(color::Reset),
                ),
                true,
            );
        }
    }

    fn print_info(&mut self, output: &str) {
        let line = &format!("[**] {}", output);
        self.print_line(line, true);
        self.tts_ctrl.lock().unwrap().speak_info(output);
    }

    fn print_error(&mut self, output: &str) {
        let line = &format!("{}[!!] {}{}", Fg(color::Red), output, Fg(color::Reset));
        self.print_line(line, true);
        self.tts_ctrl.lock().unwrap().speak_error(output);
    }

    fn scroll_to(&mut self, row: usize) -> Result<()> {
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

    fn scroll_lock(&mut self, lock: bool) -> Result<()> {
        self.scroll_data.lock(lock)
    }

    fn scroll_up(&mut self) -> Result<()> {
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

    fn scroll_down(&mut self) -> Result<()> {
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

    fn scroll_top(&mut self) -> Result<()> {
        if self.history.inner.len() as u16 >= self.output_line {
            self.init_scroll()?;
            self.scroll_data.pos = 0;
            self.draw_scroll()?;
        }
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
                ScrollRegion(OUTPUT_START_LINE, self.output_line),
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
        Ok(())
    }

    fn find_up(&mut self, pattern: &Regex) -> Result<()> {
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

    fn flush(&mut self) {
        self.screen.flush().unwrap();
    }
}

impl Screen {
    pub fn new(
        tts_ctrl: Arc<Mutex<TTSController>>,
        mouse_support: bool,
    ) -> Result<Self, Box<dyn error::Error>> {
        let screen = Self::create_screen_writer(mouse_support)?;
        let (width, height) = termion::terminal_size()?;

        let status_area_height = 1;
        let output_line = height - status_area_height - 1;
        let prompt_line = height;

        let status_area = StatusArea::new(status_area_height, output_line + 1, width);

        Ok(Self {
            screen,
            tts_ctrl,
            width,
            height,
            output_line,
            status_area,
            prompt_line,
            cursor_prompt_pos: 1,
            history: History::new(),
            scroll_data: ScrollData::new(),
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
            self.redraw_status_area()?;
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

    fn print_line(&mut self, line: &str, new_line: bool) {
        self.history.append(line);
        if self.scroll_data.not_scrolled_or_split() {
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, self.output_line),
                if new_line { "\n" } else { "" },
                &line,
                self.goto_prompt(),
            )
            .unwrap();
        }
    }

    fn redraw_top_bar(&mut self) -> Result<()> {
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::CurrentLine,
            Fg(color::Green),
        )?;
        let output = if let Some(connection) = &self.connection {
            format!("═ {} ═", connection)
        } else {
            "".to_string()
        };
        write!(self.screen, "{:═<1$}", output, self.width as usize)?; // Print separator
        write!(self.screen, "{}{}", Fg(color::Reset), self.goto_prompt(),)?;
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
            let line_no = OUTPUT_START_LINE + i;
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
                "{}{}{}{}",
                termion::cursor::Goto(1, line_no),
                termion::clear::CurrentLine,
                termion::style::Reset,
                line,
            )?;
        }
        Ok(())
    }

    /// Creates the io::Write terminal handler we draw to.
    fn create_screen_writer(mouse_support: bool) -> Result<Box<dyn Write>, Box<dyn error::Error>> {
        let screen = AlternateScreen::from(stdout().into_raw_mode()?);
        if mouse_support {
            Ok(Box::new(MouseTerminal::from(screen)))
        } else {
            Ok(Box::new(screen))
        }
    }

    fn scroll_range(&self) -> u16 {
        if self.scroll_data.allow_split && self.height > SCROLL_LIVE_BUFFER_SIZE * 2 {
            self.output_line - OUTPUT_START_LINE - SCROLL_LIVE_BUFFER_SIZE + 1
        } else {
            self.output_range()
        }
    }

    fn output_range(&self) -> u16 {
        self.output_line - OUTPUT_START_LINE + 1
    }
}

fn wrap_line(line: &str, width: usize) -> Vec<&str> {
    let mut lines: Vec<&str> = vec![];

    for line in line.lines() {
        // If the line is empty just push and continue
        if line.trim().is_empty() {
            lines.push(line);
            continue;
        }

        let mut last_cut: usize = 0;
        let mut last_space: usize = 0;
        let mut print_length = 0;
        let mut print_length_since_space = 0;
        let mut in_escape = false;
        for (length, c) in line.char_indices() {
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
                    lines.push(&line[last_cut..length + c.len_utf8()]);
                    print_length = 0;
                    last_cut = length + c.len_utf8();
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
        let lines = wrap_line(line, 11);
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
                line = format!("{}{}", line, num.repeat(15));
            }
        }
        let lines = wrap_line(&line, 15);
        assert_eq!(lines.len(), 1000 * 10);
        for (i, line) in lines.iter().enumerate() {
            let num = format!("{}", i % 10);
            assert_eq!(line, &num.repeat(15).to_string());
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

    #[test]
    fn test_search_history() {
        let line = "a nice line\n\nwith a blank line\nand lines\nc\ntest\n";

        let mut history = History::new();
        history.append(line);
        let re = crate::model::Regex::new("and lines").unwrap();
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
