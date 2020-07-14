use crate::model::Line;

use termion::{raw::RawTerminal, screen::AlternateScreen};

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{Stdout, Write};
use std::rc::Rc;

struct ScrollData(bool, usize);
type ScreenHandle = AlternateScreen<RawTerminal<Stdout>>;

#[derive(Clone)]
pub struct ScreenWriter {
    handle: Rc<RefCell<ScreenHandle>>,
}

impl ScreenWriter {
    pub fn new(screen: ScreenHandle) -> Self {
        Self {
            handle: Rc::new(RefCell::new(screen)),
        }
    }
}

impl Write for ScreenWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.handle.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.handle.borrow_mut().flush()
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

    fn len(&self) -> usize {
        self.inner.len()
    }
}

pub struct Window {
    screen: ScreenWriter,
    history: History,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    scroll_data: ScrollData,
}

impl Window {
    pub fn new(screen: ScreenWriter, x: u16, y: u16, width: u16, height: u16) -> Self {
        Window {
            screen,
            history: History::new(),
            x,
            y,
            width,
            height,
            scroll_data: ScrollData(false, 0),
        }
    }

    pub fn is_scrolled(&self) -> bool {
        self.scroll_data.0
    }

    pub fn redraw(&mut self) {
        let top = {
            if self.scroll_data.0 {
                self.scroll_data.1
            } else if self.history.len() > self.height as usize {
                self.history.len() - self.height as usize
            } else {
                0
            }
        };

        let bottom = top + self.height as usize;

        // FIXME: Wrap lines if too long
        for i in top..bottom {
            let line = self.history.inner.get(i).map(|x| x.as_str()).unwrap_or("");
            let y = (i - top + 2) as u16;
            write!(
                self.screen,
                "{}{}{:<3$}",
                termion::cursor::Goto(self.x, y as u16),
                termion::color::Fg(termion::color::Reset),
                line,
                self.width as usize,
            )
            .unwrap();
        }
    }

    pub fn print_prompt(&mut self, prompt: &Line) {
        if let Some(prompt_line) = prompt.print_line() {
            self.history.append(prompt_line);
            if !self.scroll_data.0 {
                write!(
                    self.screen,
                    "{}{}",
                    termion::cursor::Goto(self.x, self.y + self.height),
                    prompt_line,
                )
                .unwrap();
            }
        }
    }

    pub fn print_line(&mut self, line: &str) {
        self.history.append(&line);
        self.redraw();
    }

    pub fn print_output(&mut self, line: &Line) {
        if let Some(print_line) = line.print_line() {
            if print_line.trim().is_empty() {
                self.print_line(&print_line);
            } else {
                for line in wrap_line(&print_line, self.width as usize) {
                    self.print_line(&line);
                }
            }
        }
    }

    pub fn scroll_up(&mut self) {
        if self.history.len() > self.height as usize {
            if !self.scroll_data.0 {
                self.scroll_data.0 = true;
                self.scroll_data.1 = self.history.len() - self.height as usize;
            }
            self.scroll_data.0 = true;
            self.scroll_data.1 -= self.scroll_data.1.min(5);
            self.redraw();
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_data.0 {
            let max_start_index: i32 = self.history.len() as i32 - self.height as i32;
            let new_start_index = self.scroll_data.1 + 5;
            if new_start_index >= max_start_index as usize {
                self.reset_scroll();
            } else {
                self.scroll_data.1 = new_start_index;
            }
            self.redraw();
        }
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_data.0 = false;
        self.redraw();
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
mod window_test {
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
