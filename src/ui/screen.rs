use crate::{
    model::Line,
    ui::window::{ScreenWriter, Window},
};

use anyhow::Result;

use std::cell::RefCell;
use std::io::{stdout, Write};
use std::rc::Rc;
use std::{error, fmt};
use termion::{color, raw::IntoRawMode, screen::AlternateScreen};

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

    fn redraw(&mut self, screen: &mut ScreenWriter, scrolled: bool) -> Result<()> {
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

    fn draw_bar(&self, line: u16, screen: &mut ScreenWriter, custom_info: &str) -> Result<()> {
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

    fn draw_line(&self, line: u16, screen: &mut ScreenWriter, info: &str) -> Result<()> {
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

pub struct Screen {
    screen: ScreenWriter,
    pub width: u16,
    pub height: u16,
    output_line: u16,
    prompt_line: u16,
    status_area: StatusArea,
    cursor_prompt_pos: u16,
    main_window: Rc<RefCell<Window>>,
    active_window: Rc<RefCell<Window>>,
}

impl Screen {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        let screen = AlternateScreen::from(stdout().into_raw_mode()?);
        let screen_writer = ScreenWriter::new(screen);
        let (width, height) = termion::terminal_size()?;

        let status_area_height = 1;
        let output_line = height - status_area_height - 1;
        let prompt_line = height;

        let status_area = StatusArea::new(status_area_height, output_line + 1, width);

        let main_window = Window::new(
            screen_writer.clone(),
            1,
            2,
            width,
            height - status_area_height - 2,
        );
        let main_window_ref = Rc::new(RefCell::new(main_window));

        Ok(Self {
            screen: screen_writer,
            width,
            height,
            output_line,
            status_area,
            prompt_line,
            cursor_prompt_pos: 1,
            main_window: main_window_ref.clone(),
            active_window: main_window_ref,
        })
    }

    pub fn setup(&mut self) -> Result<()> {
        self.reset()?;

        // Get params in case screen resized
        let (width, height) = termion::terminal_size()?;
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            self.output_line = height - self.status_area.height() - 1;
            self.prompt_line = height;

            self.redraw_top_bar("", 0)?;
            self.status_area
                .redraw(&mut self.screen, self.active_window.borrow().is_scrolled())?;
            self.reset_scroll();
            self.screen.flush()?;
            Ok(())
        } else {
            Err(TerminalSizeError.into())
        }
    }

    pub fn set_status_area_height(&mut self, height: u16) -> Result<()> {
        let height = height.max(1).min(5);
        self.status_area.set_height(height, self.height - height);
        self.setup()?;
        Ok(())
    }

    pub fn set_status_line(&mut self, line: usize, info: String) -> Result<()> {
        self.status_area.set_status_line(line, info);
        self.status_area
            .redraw(&mut self.screen, self.active_window.borrow().is_scrolled())?;
        Ok(())
    }

    pub fn redraw_top_bar(&mut self, host: &str, port: u16) -> Result<()> {
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
        Ok(())
    }

    pub fn goto_prompt(&mut self) {
        let _ = write!(
            self.screen,
            "{}",
            termion::cursor::Goto(self.cursor_prompt_pos, self.prompt_line)
        );
    }

    pub fn reset(&mut self) -> Result<()> {
        write!(self.screen, "{}", termion::clear::All)?;
        Ok(())
    }

    pub fn print_prompt(&mut self, prompt: &Line) {
        self.main_window.borrow_mut().print_prompt(prompt);
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
            "{}{}{}",
            termion::cursor::Goto(1, self.prompt_line),
            termion::clear::CurrentLine,
            input
        )
        .unwrap();
    }

    pub fn print_output(&mut self, line: &Line) {
        self.main_window.borrow_mut().print_output(line)
    }

    pub fn print_send(&mut self, send: &Line) {
        if let Some(line) = send.print_line() {
            self.main_window.borrow_mut().print_line(&format!(
                "{}> {}{}",
                color::Fg(color::LightYellow),
                line,
                color::Fg(color::Reset)
            ));
        }
    }

    pub fn print_info(&mut self, output: &str) {
        self.main_window
            .borrow_mut()
            .print_line(&format!("[**] {}", output));
    }

    pub fn print_error(&mut self, output: &str) {
        self.main_window.borrow_mut().print_line(&format!(
            "{}[!!] {}{}",
            color::Fg(color::Red),
            output,
            color::Fg(color::Reset)
        ));
    }

    pub fn scroll_up(&mut self) {
        self.active_window.borrow_mut().scroll_up();
    }

    pub fn scroll_down(&mut self) {
        self.active_window.borrow_mut().scroll_down();
    }

    pub fn reset_scroll(&mut self) {
        self.active_window.borrow_mut().reset_scroll();
    }

    pub fn flush(&mut self) {
        self.screen.flush().unwrap();
    }
}
