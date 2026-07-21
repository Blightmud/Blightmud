use std::io::Write;

use anyhow::Result;
use termion::{
    clear,
    cursor::{self, Goto},
};

use crate::{
    model::{Line, Regex},
    tools::printable_chars::PrintableCharsIterator,
    ui::{DisableOriginMode, ResetScrollRegion, ScrollRegion},
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
        self.history.append_str(line);
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
            self.history.append_str(print_line);
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
        let width = self.width as usize;

        // Calculate display width up to cursor position (pos is character index)
        let chars_before_cursor: String = line.chars().take(pos).collect();
        let mut cursor_display_pos = chars_before_cursor.as_str().display_width();

        // Scroll the view when cursor goes past the visible width
        while input.display_width() >= width && cursor_display_pos >= width {
            let (byte_idx, skipped_width) = input.byte_index_at_display_width(width);
            if byte_idx < input.len() {
                input = input.split_at(byte_idx).1;
                cursor_display_pos -= skipped_width;
            } else {
                input = "";
                cursor_display_pos = 0;
            }
        }

        // Truncate input if it's still too wide for the display
        if input.display_width() >= width {
            let (byte_idx, _) = input.byte_index_at_display_width(width);
            input = input.split_at(byte_idx).0;
        }

        write!(
            self.screen,
            "{}{}{}{}",
            Goto(1, self.prompt_line),
            clear::CurrentLine,
            input,
            Goto(cursor_display_pos as u16 + 1, self.prompt_line)
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
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, i + 1),
                termion::clear::CurrentLine,
                self.history.get(index).line(),
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
        self.print_line(&Line::from(format!("ERROR: {output}")));
    }

    fn print_info(&mut self, output: &str) {
        self.print_line(&Line::from(format!("INFO: {output}")));
    }

    fn print_output(&mut self, line: &Line) {
        // Handle screen clear request from server
        if line.flags.screen_clear {
            self.clear_output_area().ok();
        }
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
                for l in wrap_line(print_line, self.width as usize, 0) {
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
        // Row breaks must be substituted *before* the sanitize below. The vte
        // parser behind `printable_chars` routes '\n' to `execute`, not
        // `print`, so it is silently dropped and "one\ntwo" would be read out
        // as "onetwo". This is a fix, not just a guard.
        //
        // The substitute is exactly one char of exactly one display column,
        // because `pos` indexes the pre-substitution buffer and the column
        // computed below is treated as an absolute terminal column. In reader
        // mode that column *is* the screen reader's anchor, so a one-column
        // drift is a mis-announcement rather than a cosmetic glitch. A space
        // avoids depending on how espeak, macOS `say` or NVDA pronounce a
        // glyph.
        const ROW_BREAK: char = ' ';
        debug_assert_eq!(unicode_width::UnicodeWidthChar::width(ROW_BREAK), Some(1));
        let input = input.replace('\n', &ROW_BREAK.to_string());
        let input = input.as_str();

        // Reader screens only operate on printable input characters (no term control sequences, e.g. ANSI colour).
        let sanitized_input = input.printable_chars().collect::<String>();
        let input = sanitized_input.as_str();
        let width = self.width as usize;

        // Calculate display width up to cursor position (pos is character index)
        let chars_before_cursor: String = input.chars().take(pos).collect();
        let mut display_pos = chars_before_cursor.as_str().display_width();

        if let Some((existing, orig_display_pos)) = &self.prompt_input {
            if (width - 1..width + 1).contains(&display_pos) {
                // Fall back to default behaviour when the prompt wraps
                self.print_wrapped_prompt_input(input, pos);
            } else {
                let mut orig = *orig_display_pos;
                while display_pos >= width {
                    display_pos -= width;
                    if orig >= width {
                        orig -= width;
                    }
                }
                if input.starts_with(existing) {
                    let suffix = input[existing.len()..].to_owned();
                    self.print_prompt_input_suffix(&suffix, orig, display_pos);
                } else if existing.starts_with(input) {
                    self.trim_prompt_input(display_pos);
                } else {
                    self.print_wrapped_prompt_input(input, pos);
                }
            }
        } else {
            self.print_wrapped_prompt_input(input, pos);
        }
        self.prompt_input = Some((input.to_string(), display_pos));
    }

    fn print_send(&mut self, send: &Line) {
        if self.scroll_data.active && send.flags.source != Some("script".to_string()) {
            self.reset_scroll().ok();
        }
        if let Some(print_line) = send.print_line() {
            self.history.append_str(print_line);
        }
    }

    fn reset(&mut self) -> Result<()> {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion)?;
        Ok(())
    }

    fn reset_scroll(&mut self) -> Result<()> {
        self.scroll_data.reset(&self.history)?;
        let output_range = self.output_line;
        let output_start_index = self.history.len() as i32 - output_range as i32;
        if output_start_index >= 0 {
            let output_start_index = output_start_index as usize;
            for i in 0..output_range {
                let index = output_start_index + i as usize;
                write!(
                    self.screen,
                    "{}{}{}{}",
                    cursor::Goto(1, 1 + i),
                    clear::AfterCursor,
                    self.history.get(index).line(),
                    cursor::Goto(1, self.prompt_line),
                )?;
            }
        } else {
            for line in self.history.iter() {
                write!(
                    self.screen,
                    "{}\n{}{}{}",
                    Goto(1, self.output_line),
                    clear::AfterCursor,
                    line.line(),
                    cursor::Goto(1, self.prompt_line),
                )?;
            }
        }
        Ok(())
    }

    fn clear_output_area(&mut self) -> Result<()> {
        // Clear all lines in the output area
        for line_no in 1..=self.output_line {
            write!(
                self.screen,
                "{}{}",
                cursor::Goto(1, line_no),
                clear::CurrentLine,
            )?;
        }
        // Clear the history buffer
        self.history.clear();
        // Reset scroll state
        self.scroll_data.reset(&self.history)?;
        // Reposition cursor
        write!(self.screen, "{}", cursor::Goto(1, self.prompt_line))?;
        Ok(())
    }

    fn scroll_down(&mut self) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        if self.scroll_data.active {
            let output_range = self.output_line as i32;
            let max_start_index = self.history.len() as i32 - output_range;
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
            let max_start_index = self.history.len() as i32 - self.output_line as i32;
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
        if self.history.len() as u16 >= self.output_line {
            self.scroll_data.active = true;
            self.scroll_data.pos = 0;
            self.draw_scroll()?;
        }
        Ok(())
    }

    fn scroll_up(&mut self) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        let output_range = self.output_line as usize;
        if self.history.len() > output_range {
            if !self.scroll_data.active {
                self.scroll_data.active = true;
                self.scroll_data.pos = self.history.len() - output_range;
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
            self.scroll_to(0.max(line))?;
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

    fn remove_tag(&mut self, _: &str) -> Result<()> {
        Ok(())
    }

    fn clear_tags(&mut self) -> Result<()> {
        Ok(())
    }

    fn set_status_area_height(&mut self, _height: u16) -> Result<()> {
        Ok(())
    }

    /// Reader mode stays one row tall: a screen reader gains nothing from
    /// a taller box, and the minimal-diff logic assumes a single-line
    /// model. Row breaks are rendered as a marker instead.
    fn set_input_height(&mut self, _height: u16) -> Result<()> {
        Ok(())
    }

    fn input_height(&self) -> u16 {
        1
    }

    fn set_show_tags(&mut self, _show: bool) -> Result<()> {
        Ok(())
    }

    fn set_tag_mask(&mut self, _mask: crate::model::TagMask) {}

    fn set_history_capacity(&mut self, capacity: usize) {
        self.history.set_capacity(capacity);
    }

    fn set_status_line(&mut self, _line: usize, _info: String) -> Result<()> {
        Ok(())
    }

    fn set_top_line(&mut self, _info: Option<String>) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_top_row(
        &mut self,
        _selector: crate::ui::TopRowSelector,
        _opts: crate::ui::TopRowOpts,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn reset_top_row(&mut self, _selector: crate::ui::TopRowSelector) -> anyhow::Result<()> {
        Ok(())
    }

    fn add_top_row(&mut self, _opts: crate::ui::TopRowOpts) -> anyhow::Result<()> {
        Ok(())
    }

    fn remove_top_row(&mut self, _selector: crate::ui::TopRowSelector) -> anyhow::Result<()> {
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

    fn swap_history(&mut self, new: super::history::History) -> Result<super::history::History> {
        self.scroll_data = ScrollData::new();
        let old = std::mem::replace(&mut self.history, new);
        self.setup()?;
        Ok(old)
    }

    fn set_tab_indicator(&mut self, _tabs: Vec<crate::tabs::TabInfo>) -> Result<()> {
        // Reader mode skips the tab indicator (it's a screen-reader-friendly
        // single-stream view). The active tab still drives what's read.
        Ok(())
    }
}

#[cfg(test)]
mod reader_screen_test {
    use super::*;
    use std::sync::{Arc, Mutex};

    // `ReaderScreen::new` needs a real terminal, but `print_prompt_input` only
    // needs `&mut self` and a `Box<dyn Write>`, so a capturing writer gets the
    // diff logic under test without a TTY. Reader mode had no automated
    // coverage at all before this; an untested path is a second-class path
    // regardless of intent.

    #[derive(Clone)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn test_screen() -> (ReaderScreen, Arc<Mutex<Vec<u8>>>) {
        let sink = Arc::new(Mutex::new(Vec::new()));
        let screen = ReaderScreen {
            screen: Box::new(SharedBuf(sink.clone())),
            history: History::new(),
            scroll_data: ScrollData::new(),
            output_line: 23,
            prompt_line: 24,
            width: 40,
            height: 24,
            prompt_input: None,
        };
        (screen, sink)
    }

    fn rendered(sink: &Arc<Mutex<Vec<u8>>>) -> String {
        String::from_utf8(sink.lock().unwrap().clone()).unwrap()
    }

    /// vte routes '\n' to `execute`, not `print`, so `printable_chars` drops it
    /// silently — two rows would run together as one word. The substitution has
    /// to happen before that sanitize.
    #[test]
    fn row_breaks_are_substituted_not_dropped() {
        let (mut screen, sink) = test_screen();
        screen.print_prompt_input("one\ntwo", 7);

        let out = rendered(&sink);
        assert!(
            out.contains("one two"),
            "row break was dropped, giving {out:?}"
        );
        assert!(!out.contains("onetwo"));
    }

    /// The substitute must be one character of one column. `pos` indexes the
    /// pre-substitution buffer and the column below is treated as an absolute
    /// terminal column, which in reader mode is the screen reader's anchor —
    /// a one-column drift is a mis-announcement, not a cosmetic glitch.
    #[test]
    fn row_break_does_not_shift_the_cursor_column() {
        let (mut screen, _sink) = test_screen();
        screen.print_prompt_input("one\ntwo", 7);
        let (_, with_break) = screen.prompt_input.clone().unwrap();

        let (mut screen, _sink) = test_screen();
        screen.print_prompt_input("one two", 7);
        let (_, without_break) = screen.prompt_input.clone().unwrap();

        assert_eq!(with_break, without_break);
    }

    /// Reader mode stays one row tall whatever is requested, and says so.
    #[test]
    fn input_height_is_always_one() {
        let (mut screen, _sink) = test_screen();
        assert_eq!(screen.input_height(), 1);
        screen.set_input_height(5).unwrap();
        assert_eq!(screen.input_height(), 1);
    }
}
