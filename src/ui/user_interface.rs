use std::{error, fmt, io::Write};

#[cfg(test)]
use mockall::automock;

use crate::model::{Line, Regex, TagMask};
use crate::tabs::TabInfo;
use crate::tools::printable_chars::PrintableCharsIterator;

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
    fn set_input_height(&mut self, height: u16) -> Result<()>;
    /// The number of rows the input area *actually* occupies.
    ///
    /// Deliberately a report rather than an echo of what Lua requested.
    /// The two diverge on a terminal too short to honour the request, and
    /// in reader mode, where the setter is a no-op — and NAWS reports this
    /// to the MUD, so an echo would corrupt server-side wrapping for
    /// precisely the users who hear the artifacts read aloud.
    fn input_height(&self) -> u16;
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
        let mut print_length = 0;
        let mut print_length_since_space = 0;
        for (length, c) in line.printable_char_indices() {
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

/// Hard-wrap a single line to `width` display columns, **losslessly**.
///
/// This is deliberately a second wrapper rather than an option on
/// [`wrap_line`]. `wrap_line` is tuned for MUD *output* and is lossy by
/// design: it breaks on word boundaries and drops the boundary space, and it
/// discards a trailing segment that is entirely whitespace. That is right for
/// prose and wrong for an input area, where the wrapped rows have to partition
/// the buffer exactly — a dropped space shifts every character after it, so
/// the cursor would render a column away from the text it edits.
///
/// The returned slices concatenate back to `line` byte for byte. Breaks only
/// ever land on printable-character boundaries, so an escape sequence is never
/// split; escapes are carried into whatever row they start in and consume no
/// columns. A consequence, accepted rather than fixed: an SGR run that spans a
/// break is not re-emitted on the continuation row, so styling stops there.
///
/// `line` must not contain `'\n'`. The vte parser behind
/// `printable_char_indices` routes `'\n'` to `execute`, not `print`, so it is
/// invisible here and two logical rows would silently be measured as one.
/// Split on `'\n'` first, then call this on each segment.
pub fn wrap_line_hard(line: &str, width: usize) -> Vec<&str> {
    debug_assert!(
        !line.contains('\n'),
        "wrap_line_hard cannot see '\\n'; split on it first"
    );

    // A zero width would make every row empty and the loop never advance.
    let width = width.max(1);
    let mut rows: Vec<&str> = vec![];
    let mut start = 0;

    while start < line.len() {
        let rest = &line[start..];
        let (mut cut, _) = rest.byte_index_at_display_width(width);

        if cut == 0 {
            // The next glyph is wider than the whole row — a double-width
            // character at `width == 1`, say. Nothing fits, but the row must
            // still consume something or this loop spins forever. Overflow the
            // row by one character, which is also what a terminal does.
            cut = match rest.printable_char_indices().next() {
                Some((idx, c)) => idx + c.len_utf8(),
                // No printable characters at all (a bare escape sequence);
                // take the remainder so the bytes are not lost.
                None => rest.len(),
            };
        }

        rows.push(&rest[..cut]);
        start += cut;
    }

    // An empty input is one empty row, not zero rows — there is still a row
    // the cursor sits on.
    if rows.is_empty() {
        rows.push(line);
    }

    rows
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

    /// The reason a second wrapper exists at all: `wrap_line` is lossy, and an
    /// input area cannot be. Both properties are asserted side by side so the
    /// justification cannot quietly stop being true.
    #[test]
    fn wrap_line_hard_preserves_all_characters() {
        for line in [
            "the quick brown fox jumps over the lazy dog",
            "a b c d e f g h i j k l m n o p",
            "trailing space kept   ",
            "   leading space kept",
            "no-spaces-at-all-in-this-very-long-token-here",
            "",
            " ",
        ] {
            for width in 1..=20usize {
                let rows = wrap_line_hard(line, width);
                assert_eq!(rows.concat(), line, "lossy at width {width} for {line:?}");
            }
        }

        // And the contrast: where `wrap_line` takes its word-break path it
        // drops the boundary space, so it fails the same round-trip. That is
        // correct for MUD output, and is why it is left alone.
        let line = "aaa bbb ccc";
        assert_eq!(wrap_line(line, 6, 0), vec!["aaa", "bbb", "ccc"]);
        assert_eq!(wrap_line(line, 6, 0).concat(), "aaabbbccc");
        assert_eq!(wrap_line_hard(line, 6).concat(), line);
    }

    /// Every row must fit the terminal, except when a single glyph cannot.
    #[test]
    fn wrap_line_hard_rows_fit_the_width() {
        let line = "hello 中文 world";
        for width in 2..=20usize {
            for row in wrap_line_hard(line, width) {
                assert!(
                    row.display_width() <= width,
                    "row {row:?} exceeds width {width}"
                );
            }
        }
    }

    /// A double-width glyph in a one-column terminal fits nowhere. The row has
    /// to overflow rather than the wrap loop spinning forever.
    #[test]
    fn wrap_line_hard_terminates_on_glyph_wider_than_terminal() {
        let rows = wrap_line_hard("中文", 1);
        assert_eq!(rows, vec!["中", "文"]);
        assert_eq!(rows.concat(), "中文");
    }

    /// Breaks land on printable-character boundaries, so an escape sequence is
    /// never cut in half — a split escape would be printed as literal garbage.
    #[test]
    fn wrap_line_hard_never_splits_an_escape_sequence() {
        let line = "\x1b[31mred\x1b[0m and \x1b[32mgreen\x1b[0m";
        for width in 1..=20usize {
            let rows = wrap_line_hard(line, width);
            assert_eq!(rows.concat(), line);
            for row in rows {
                // A row holding a partial CSI would have an unterminated
                // `\x1b[` with no final byte.
                if let Some(esc) = row.rfind('\x1b') {
                    assert!(
                        row[esc..].chars().any(|c| c.is_ascii_alphabetic()),
                        "row {row:?} ends mid-escape"
                    );
                }
            }
        }
    }

    /// Escapes consume no columns, so they must not push text onto a new row.
    #[test]
    fn wrap_line_hard_escapes_do_not_consume_columns() {
        let rows = wrap_line_hard("\x1b[31mabcde\x1b[0m", 5);
        assert_eq!(rows.len(), 1);
    }

    /// Empty input is one row, not zero — the cursor still sits somewhere.
    #[test]
    fn wrap_line_hard_empty_input_is_one_row() {
        assert_eq!(wrap_line_hard("", 10), vec![""]);
    }
}
