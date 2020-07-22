use crate::{
    model::Line,
    ui::ansi::{parse_ansi, OwnedSpans},
};
use anyhow::Result;
use std::collections::VecDeque;
use std::io::{stdout, Stdout};
use std::{error, fmt};
use termion::{
    color,
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
};
use tui::backend::TermionBackend;
use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Style};
use tui::terminal::Terminal;
use tui::text::{Spans, Text};
use tui::widgets::{Block, BorderType, Borders, Paragraph, Widget};

struct ScrollData(bool, usize);

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
    scrolled: bool,
    status_lines: Vec<Option<String>>,
}

impl Widget for &StatusArea {
    fn render(self, area: Rect, buf: &mut Buffer) {
        fn draw_bar<T: AsRef<str>>(
            area: Rect,
            scrolled: bool,
            content: &Option<T>,
            buffer: &mut Buffer,
        ) {
            let bar_color = Style::default().fg(Color::Green);
            let more = if scrolled { "━ (more) " } else { "" };

            buffer.set_string(area.x, area.y, &more, bar_color);

            if let Some(content) = content {
                let spans = parse_ansi(content.as_ref()).remove(0);
                let span_len = spans.len();
                if more.len() + 2 + span_len + 1 >= area.width as usize {
                    // No space to draw. Abort
                    return;
                }
                let bar_width = area.width as usize - (more.len() + 2 + span_len + 1);

                buffer.set_string(area.x + more.len() as u16, area.y, "━ ", bar_color);

                buffer.set_spans(
                    area.x + more.len() as u16 + 2,
                    area.y,
                    &spans.into(),
                    span_len as u16,
                );

                buffer.set_string(
                    area.x + more.len() as u16 + 2 + span_len as u16,
                    area.y,
                    format!(" {:━<1$}", "", bar_width),
                    bar_color,
                );
            } else {
                buffer.set_string(
                    area.x + more.len() as u16,
                    area.y,
                    format!("{:━<1$}", "", area.width as usize - more.len()),
                    bar_color,
                );
            }
        }

        // Draw first line
        draw_bar(area, self.scrolled, &self.status_lines[0], buf);

        let mut remaining_height = area.height;
        if self.height() > 1 {
            // Draw non-bar lines
            for line_no in 1..(self.height() - 1) {
                if remaining_height == 0 {
                    break;
                }
                remaining_height -= 1;

                if let Some(line) = &self.status_lines[line_no as usize] {
                    let spans = parse_ansi(line).remove(0);
                    let spans_len = spans.len();
                    buf.set_spans(area.x, area.y + line_no, &spans.into(), spans_len as u16);
                }
            }

            // Draw last line
            if remaining_height > 0 {
                let last_line_area = {
                    let mut rect = area;
                    rect.y = rect.y + self.height() - 1;
                    rect
                };
                draw_bar(
                    last_line_area,
                    false,
                    &self.status_lines[self.height() as usize - 1],
                    buf,
                );
            }
        }
    }
}

impl StatusArea {
    fn new(height: u16) -> Self {
        Self {
            scrolled: false,
            status_lines: vec![None; height as usize],
        }
    }

    fn set_status_line(&mut self, index: usize, line: String) {
        let index = index.max(0).min(self.status_lines.len() - 1);
        if !line.trim().is_empty() {
            self.status_lines[index] = Some(line);
        } else {
            self.status_lines[index] = None;
        }
    }

    fn set_height(&mut self, height: u16) {
        self.status_lines.resize(height as usize, None);
    }

    fn height(&self) -> u16 {
        self.status_lines.len() as u16
    }
}

struct History {
    inner: VecDeque<OwnedSpans>,
}

impl History {
    fn new() -> Self {
        Self {
            inner: VecDeque::with_capacity(32 * 1024),
        }
    }

    fn append_spans(&mut self, spans: OwnedSpans) {
        self.inner.push_back(spans);
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

type Backend = TermionBackend<AlternateScreen<RawTerminal<Stdout>>>;

pub struct Screen {
    terminal: Terminal<Backend>,
    pub width: u16,
    pub height: u16,
    status_area: StatusArea,
    cursor_prompt_pos: u16,
    history: History,
    scroll_data: ScrollData,
    title: String,
    prompt_text: String,
}

impl Screen {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        let backend = TermionBackend::new(AlternateScreen::from(stdout().into_raw_mode()?));
        let terminal = Terminal::new(backend)?;
        let (width, height) = termion::terminal_size()?;

        let status_area_height = 1;

        let status_area = StatusArea::new(status_area_height);

        Ok(Self {
            terminal,
            width,
            height,
            status_area,
            cursor_prompt_pos: 0,
            history: History::new(),
            scroll_data: ScrollData(false, 0),
            title: "".to_string(),
            prompt_text: "".to_string(),
        })
    }

    /// Initialize the screen
    pub fn setup(&mut self) -> Result<()> {
        self.clear()?;

        // Get params in case screen resized
        let size = self.terminal.size()?;
        if size.width > 0 && size.height > 0 {
            self.width = size.width;
            self.height = size.height;

            self.reset_scroll();
            self.goto_prompt()?;
            Ok(())
        } else {
            Err(TerminalSizeError.into())
        }
    }

    pub fn update_size(&mut self) -> Result<()> {
        let size = self.terminal.size()?;
        self.width = size.width;
        self.height = size.height;
        Ok(())
    }

    pub fn redraw(&mut self) -> Result<()> {
        let main_window = {
            let mut block = Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::Green))
                .border_type(BorderType::Double);
            if !self.title.is_empty() {
                let title = format!("══ {} ", self.title);
                block = block.title(title);
            }
            let mut texts: Vec<OwnedSpans> = Vec::with_capacity(self.height as usize);

            if self.status_area.height() + 2 >= self.height {
                // No space to draw. Abort
                return Ok(());
            }

            let height = self.height - 2 - self.status_area.height();

            let top = {
                if self.scroll_data.0 {
                    self.scroll_data.1
                } else if self.history.len() > height as usize {
                    self.history.len() - height as usize
                } else {
                    0
                }
            };

            let bottom = top + height as usize;

            for i in top..bottom {
                let line: OwnedSpans = self
                    .history
                    .inner
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| "".into());
                texts.push(line);
            }
            let text_spans: Vec<Spans<'_>> = texts.into_iter().map(|x| x.into()).collect();
            Paragraph::new(Text::from(text_spans))
                .block(block)
                .wrap(tui::widgets::Wrap { trim: false })
        };

        let input_prompt = Paragraph::new(Text::from(self.prompt_text.as_str()));

        let size = self.terminal.size()?;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(1),
                    Constraint::Length(self.status_area.height()),
                    Constraint::Length(1),
                ]
                .as_ref(),
            );

        let status_area = &self.status_area;

        self.terminal.draw(|frame| {
            let chunks = layout.split(size);
            frame.render_widget(main_window, chunks[0]);
            frame.render_widget(status_area, chunks[1]);
            frame.render_widget(input_prompt, chunks[2]);
        })?;

        self.goto_prompt()?;
        self.terminal.show_cursor()?;

        Ok(())
    }

    pub fn set_status_area_height(&mut self, height: u16) -> Result<()> {
        let height = height.max(1).min(5);
        self.status_area.set_height(height);
        self.redraw()?;
        Ok(())
    }

    pub fn set_status_line(&mut self, line: usize, info: String) -> Result<()> {
        self.status_area.set_status_line(line, info);
        self.redraw()?;
        Ok(())
    }

    /// Move cursor to the prompt at the bottom
    fn goto_prompt(&mut self) -> Result<()> {
        self.terminal
            .set_cursor(self.cursor_prompt_pos, self.height)?;
        Ok(())
    }

    /// Clear the screen
    pub fn clear(&mut self) -> Result<()> {
        self.terminal.clear()?;
        Ok(())
    }

    pub fn print_output(&mut self, line: &Line) {
        if let Some(print_line) = line.print_line() {
            self.print_line(&print_line);
        }
    }

    fn print_line(&mut self, line: &str) {
        for line in parse_ansi(line) {
            self.history.append_spans(line);
        }
        let _ = self.redraw();
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
        let output_range = self.height as usize - 2 - self.status_area.height() as usize;
        let history = &self.history.inner;
        if history.len() > output_range {
            if !self.scroll_data.0 {
                self.scroll_data.0 = true;
                self.scroll_data.1 = history.len() - output_range;
            }
            self.scroll_data.0 = true;
            self.scroll_data.1 -= self.scroll_data.1.min(5);
        }
        Ok(())
    }

    pub fn scroll_down(&mut self) -> Result<()> {
        if self.scroll_data.0 {
            let output_range = self.height as i32 - 2 - self.status_area.height() as i32;
            let max_start_index: i32 = self.history.inner.len() as i32 - output_range;
            let new_start_index = self.scroll_data.1 + 5;
            if new_start_index >= max_start_index as usize {
                self.reset_scroll();
            } else {
                self.scroll_data.1 = new_start_index;
            }
        }
        Ok(())
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_data.0 = false;
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = title.into();
    }

    pub fn set_prompt<T: Into<String>>(&mut self, prompt: T) {
        self.prompt_text = prompt.into();
        let _ = self.redraw();
    }

    pub fn set_cursor_pos(&mut self, pos: usize) -> Result<()> {
        self.cursor_prompt_pos = pos as u16;
        self.goto_prompt()?;
        Ok(())
    }
}

#[cfg(test)]
mod screen_test {
    use super::*;

    #[test]
    fn test_append_history() {
        let lines = parse_ansi("a nice line\n\nwith a blank line\nand lines\nc\ntest\n");

        let mut history = History::new();
        for line in lines {
            history.append_spans(line);
        }
        assert_eq!(
            history.inner,
            vec![
                OwnedSpans::from("a nice line"),
                OwnedSpans::from(""),
                OwnedSpans::from("with a blank line"),
                OwnedSpans::from("and lines"),
                OwnedSpans::from("c"),
                OwnedSpans::from("test"),
            ]
        );
    }
}
