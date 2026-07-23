use std::{error, fmt, io::Write};

#[cfg(test)]
use mockall::automock;

use crate::model::{Line, Regex, TagMask};
use crate::tabs::TabInfo;
use crate::tools::printable_chars::PrintableCharsIterator;
use unicode_width::UnicodeWidthChar;

use anyhow::Result;

use super::history::History;
use super::top_area::{TopRowOpts, TopRowSelector};

#[derive(Debug)]
pub struct TerminalSizeError;

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

#[cfg_attr(test, automock)]
pub trait UserInterface {
    fn setup(&mut self) -> Result<()>;
    fn print_error(&mut self, output: &str);
    fn print_info(&mut self, output: &str);
    fn print_output(&mut self, line: &Line);
    fn print_prompt(&mut self, prompt: &Line);
    fn print_prompt_input(&mut self, input: &str, pos: usize);
    fn print_send(&mut self, send: &Line);
    fn reset(&mut self) -> Result<()>;
    fn reset_scroll(&mut self) -> Result<()>;
    /// Clears the output area (scroll region) without affecting the rest of the UI.
    /// Called when the server sends screen-clearing escape sequences (ED sequences).
    fn clear_output_area(&mut self) -> Result<()>;
    fn scroll_down(&mut self) -> Result<()>;
    fn scroll_lock(&mut self, lock: bool) -> Result<()>;
    fn scroll_to(&mut self, row: usize) -> Result<()>;
    fn scroll_top(&mut self) -> Result<()>;
    fn scroll_up(&mut self) -> Result<()>;
    fn find_up(&mut self, pattern: &Regex) -> Result<()>;
    fn find_down(&mut self, pattern: &Regex) -> Result<()>;
    fn set_host(&mut self, host: &str, port: u16) -> Result<()>;
    fn add_tag(&mut self, proto: &str) -> Result<()>;
    fn remove_tag(&mut self, proto: &str) -> Result<()>;
    fn clear_tags(&mut self) -> Result<()>;
    fn set_status_area_height(&mut self, height: u16) -> Result<()>;
    fn set_show_tags(&mut self, show: bool) -> Result<()>;
    fn set_tag_mask(&mut self, mask: TagMask);
    fn set_history_capacity(&mut self, capacity: usize);
    fn set_status_line(&mut self, line: usize, info: String) -> Result<()>;
    fn set_top_line(&mut self, info: Option<String>) -> Result<()>;
    /// Mutate fields on an existing top row.
    fn set_top_row(&mut self, selector: TopRowSelector, opts: TopRowOpts) -> Result<()>;
    /// Reset a built-in top row's body to its dynamic default.
    fn reset_top_row(&mut self, selector: TopRowSelector) -> Result<()>;
    /// Append a new top row. Returns the assigned name when applicable.
    fn add_top_row(&mut self, opts: TopRowOpts) -> Result<()>;
    /// Remove a Lua-added top row. Built-ins refuse removal.
    fn remove_top_row(&mut self, selector: TopRowSelector) -> Result<()>;
    fn flush(&mut self);
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn destroy(self: Box<Self>) -> Result<(Box<dyn Write>, History)>;
    /// Swap the screen's scrollback History with the supplied one and
    /// trigger a redraw of the output area. Used by the tabs feature to
    /// switch which tab's scrollback is being displayed.
    ///
    /// Returns the History that was previously installed.
    fn swap_history(&mut self, new: History) -> Result<History>;
    /// Update the tab indicator with a fresh snapshot of all tabs.
    ///
    /// Called by the event handler whenever a tab is created, switched,
    /// labeled, or has its unread counter bumped. The screen stores the
    /// snapshot and re-renders the indicator row. Pass an empty `Vec` to
    /// hide the indicator.
    fn set_tab_indicator(&mut self, tabs: Vec<TabInfo>) -> Result<()>;
}

pub fn wrap_line(line: &str, width: usize, padding: usize) -> Vec<&str> {
    let width = width.saturating_sub(padding);
    let mut lines: Vec<&str> = vec![];

    for line in line.lines() {
        // If the line is empty just push and continue
        if line.trim().is_empty() {
            lines.push(line);
            continue;
        }

        let mut last_cut: usize = 0;
        let mut last_space: usize = 0;
        let mut pos: usize = 0; // Current position in display width
        let mut space_pos: usize = 0; // Position of last space in display width
        let mut space_char_pos: usize = 0; // Byte position of last space

        for (byte_pos, c) in line.printable_char_indices() {
            // Calculate the display width of this character
            let char_width = if c == '\t' {
                // For tab characters, calculate width to next tab stop
                // Standard tab stop is 8 characters
                let tab_stop = 8;
                let current_in_tab_stop = pos % tab_stop;
                if current_in_tab_stop == 0 {
                    tab_stop
                } else {
                    tab_stop - current_in_tab_stop
                }
            } else {
                // For all other characters, use their Unicode width
                c.width().unwrap_or(0)
            };

            // Check if adding this character would exceed the width
            if pos + char_width > width {
                // If we found a space to wrap at, use it
                if space_char_pos > last_cut && space_pos > 0 {
                    lines.push(&line[last_cut..space_char_pos]);
                    // Reset position to what remains after the space
                    pos = pos - space_pos;
                    last_cut = space_char_pos + 1;
                    space_pos = 0;
                    space_char_pos = 0;
                } else {
                    // No space found, wrap before current character
                    lines.push(&line[last_cut..byte_pos]);
                    // Reset position for remaining characters (starting with current char)
                    pos = char_width;
                    last_cut = byte_pos;
                    space_pos = 0;
                    space_char_pos = 0;
                }
                // Continue processing the current character in the new line
            } else {
                // Character fits, add its width to position
                pos += char_width;

                // Track space positions for word wrapping
                if c == ' ' && pos < width {
                    space_pos = pos;
                    space_char_pos = byte_pos;
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
mod tests {
    use super::*;

    #[test]
    fn test_wrap_line() {
        let line: &'static str =
            "\x1b[34mSomething \x1b[0mthat's pretty \x1b[32mlong and annoying\x1b[0m";
        let lines = wrap_line(line, 11, 0);
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&"\u{1b}[34mSomething"));
        assert_eq!(iter.next(), Some(&"\u{1b}[0mthat's"));
        assert_eq!(iter.next(), Some(&"pretty"));
        assert_eq!(iter.next(), Some(&"\u{1b}[32mlong and"));
        assert_eq!(iter.next(), Some(&"annoying\u{1b}[0m"));
    }

    #[test]
    fn test_wrap_line_with_padding() {
        // "hello world!!" is 13 printable chars.
        // At width=14 with no padding it fits on one line.
        // With padding=2 the effective width is 12, so it must wrap at the space.
        let line = "hello world!!";
        let lines = wrap_line(line, 14, 0);
        assert_eq!(lines.len(), 1);
        let lines = wrap_line(line, 14, 2);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello");
        assert_eq!(lines[1], "world!!");
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
        let lines = wrap_line(&line, 15, 0);
        assert_eq!(lines.len(), 1000 * 10);
        for (i, line) in lines.iter().enumerate() {
            let num = format!("{}", i % 10);
            assert_eq!(line, &num.repeat(15).to_string());
        }
    }

    #[test]
    fn test_wrap_line_with_osc8_hyperlink() {
        // Simulates mdcat OSC 8 hyperlink output: ESC]8;;url ESC\ visible_text ESC]8;; ESC\
        let line = "Visit \x1b]8;;https://example.com\x1b\\\x1b[34mhttps://example.com\x1b[0m\x1b]8;;\x1b\\ for info";
        let lines = wrap_line(line, 80, 0);
        // The entire line fits in 80 columns (printable: "Visit https://example.com for info" = 34 chars)
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], line);
    }

    #[test]
    fn test_wrap_line_osc8_not_eaten() {
        // With the old SGR-only parser, everything between ESC] and the next 'm' would be eaten.
        // This test ensures the visible link text is counted toward print width.
        let link = "\x1b]8;;http://x.co\x1b\\click here\x1b]8;;\x1b\\";
        // "click here" = 10 printable chars; at width 5 it must wrap.
        let lines = wrap_line(link, 5, 0);
        // 2 pieces: OSC-open + "click", " here" — trailing OSC-close escape bytes
        // are emitted as a separate segment (zero printable width).
        assert_eq!(lines.len(), 3);
        assert!(lines[0].ends_with("click"));
        assert!(lines[1].contains("here"));
    }

    #[test]
    fn test_wrap_line_csi_non_sgr() {
        // CSI sequences other than SGR (e.g. cursor movement ESC[H, erase ESC[K)
        // should also be skipped.
        let line = "\x1b[Hsome text\x1b[K";
        let lines = wrap_line(line, 80, 0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], line);
    }

    #[test]
    fn test_wrap_line_charset_designation() {
        // Charset designation sequences: ESC ( B (ASCII), ESC ) 0 (DEC Special Graphics),
        // ESC * A, ESC + C, ESC % @ etc. — should all be skipped without consuming
        // visible text.
        let line = "\x1b(Bhello \x1b)0world\x1b*A!\x1b+C\x1b%@";
        let lines = wrap_line(line, 80, 0);
        // Printable: "hello world!" = 12 chars, fits in 80 columns
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], line);
    }

    #[test]
    fn test_wrap_line_charset_designation_wrap() {
        // Ensure charset designation sequences don't affect wrap width calculation.
        let line = "\x1b(Babcde\x1b)0fghij";
        // "abcdefghij" = 10 printable chars; at width 5 it must wrap.
        let lines = wrap_line(line, 5, 0);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_wrap_line_tabs() {
        // Test tab handling - tab should advance to next 8-column boundary
        let line = "hello\tworld"; // "hello" (5) + tab (to col 8: 3 spaces) + "world" (5) = 13 total
        // At width 10: should wrap after "hello\t" (position 8)
        let lines = wrap_line(line, 10, 0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello\t"); // First line: "hello" + tab
        assert_eq!(lines[1], "world");   // Second line: "world"

        // At width 15: should fit on one line (5 + 3 + 5 = 13 < 15)
        let lines = wrap_line(line, 15, 0);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hello\tworld");

        // Test tab at beginning
        let line = "\tstart"; // tab (to col 8: 8 spaces) + "start" (5) = 13 total
        // At width 10: should wrap after tab (position 8)
        let lines = wrap_line(line, 10, 0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "\t"); // First line: just tab
        assert_eq!(lines[1], "start"); // Second line: "start"

        // Test multiple tabs
        let line = "a\tb\tc"; // "a"(1) + tab(to 8:7) + "b"(1) + tab(to 16:7) + "c"(1) = 1+7+1+7+1=17
        // At width 10: "a\tb" (1+7+1=9) fits, next tab would go to 16>10, so wrap after "b"
        let lines = wrap_line(line, 10, 0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "a\tb"); // First line: "a" + tab + "b"
        assert_eq!(lines[1], "\tc");   // Second line: tab + "c" (tab at pos 0 -> width 8)
    }
}
