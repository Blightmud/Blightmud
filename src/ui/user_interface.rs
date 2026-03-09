use std::{error, fmt, io::Write};

#[cfg(test)]
use mockall::automock;

use crate::model::{Line, Regex};

use anyhow::Result;

use super::history::History;

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
    fn set_status_line(&mut self, line: usize, info: String) -> Result<()>;
    fn flush(&mut self);
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn destroy(self: Box<Self>) -> Result<(Box<dyn Write>, History)>;
}

/// Tracks the parser state for ANSI/VT escape sequences during line wrapping.
/// This allows wrap_line to correctly skip over all escape sequences when
/// calculating printable line width, not just SGR (\x1b[...m) sequences.
#[derive(Debug, Clone, Copy, PartialEq)]
enum EscapeState {
    /// Normal text — characters contribute to print width.
    Ground,
    /// Saw ESC (\x1b), waiting to see what kind of sequence follows.
    Escape,
    /// Inside a CSI sequence (\x1b[...) — terminated by any byte in 0x40..=0x7E.
    Csi,
    /// Inside an OSC sequence (\x1b]...) — terminated by BEL (\x07) or ST (\x1b\\).
    Osc,
    /// Inside an OSC sequence, just saw ESC — might be the ST terminator (\x1b\\).
    OscEscSeen,
    /// Inside a DCS (\x1bP), SOS (\x1bX), PM (\x1b^), or APC (\x1b_) string —
    /// terminated by ST (\x1b\\).
    StringSeq,
    /// Inside a string sequence, just saw ESC — might be the ST terminator.
    StringEscSeen,
    /// Saw ESC followed by an intermediate byte (e.g. '(', ')', '*', '+', '%')
    /// for charset designation sequences like ESC ( B, ESC ) 0, etc.
    /// Consumes the next (final) byte and returns to Ground.
    EscapeIntermediate,
}

pub fn wrap_line(line: &str, width: usize) -> Vec<&str> {
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
        let mut state = EscapeState::Ground;
        for (length, c) in line.char_indices() {
            match state {
                EscapeState::Ground => {
                    if c == '\x1b' {
                        state = EscapeState::Escape;
                        continue;
                    }
                }
                EscapeState::Escape => {
                    match c {
                        '[' => {
                            state = EscapeState::Csi;
                            continue;
                        }
                        ']' => {
                            state = EscapeState::Osc;
                            continue;
                        }
                        // DCS, SOS, PM, APC — string sequences terminated by ST
                        'P' | 'X' | '^' | '_' => {
                            state = EscapeState::StringSeq;
                            continue;
                        }
                        // Charset designation sequences: ESC ( F, ESC ) F, ESC * F,
                        // ESC + F, ESC % F — intermediate byte + one final byte.
                        '(' | ')' | '*' | '+' | '%' => {
                            state = EscapeState::EscapeIntermediate;
                            continue;
                        }
                        // Two-character escape sequences (e.g. ESC c, ESC 7, ESC 8, etc.)
                        // and any other ESC + single byte — consume and return to ground.
                        _ => {
                            state = EscapeState::Ground;
                            continue;
                        }
                    }
                }
                EscapeState::EscapeIntermediate => {
                    // Consume the final byte of the charset designation sequence
                    // (e.g. 'B' in ESC ( B, '0' in ESC ) 0) and return to ground.
                    state = EscapeState::Ground;
                    continue;
                }
                EscapeState::Csi => {
                    // CSI parameters and intermediates: 0x20..=0x3F
                    // Final byte: 0x40..=0x7E (any ASCII letter or @[\]^_`{|}~)
                    if c as u32 >= 0x40 && c as u32 <= 0x7E {
                        state = EscapeState::Ground;
                    }
                    continue;
                }
                EscapeState::Osc => {
                    if c == '\x07' {
                        // BEL terminates OSC
                        state = EscapeState::Ground;
                    } else if c == '\x1b' {
                        // Might be start of ST (\x1b\\)
                        state = EscapeState::OscEscSeen;
                    }
                    continue;
                }
                EscapeState::OscEscSeen => {
                    // After ESC inside OSC: if '\' then ST terminates, otherwise
                    // stay in OSC (the ESC was part of the OSC content).
                    if c == '\\' {
                        state = EscapeState::Ground;
                    } else {
                        state = EscapeState::Osc;
                    }
                    continue;
                }
                EscapeState::StringSeq => {
                    if c == '\x1b' {
                        state = EscapeState::StringEscSeen;
                    }
                    continue;
                }
                EscapeState::StringEscSeen => {
                    if c == '\\' {
                        state = EscapeState::Ground;
                    } else {
                        state = EscapeState::StringSeq;
                    }
                    continue;
                }
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
mod tests {
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
    fn test_wrap_line_with_osc8_hyperlink() {
        // Simulates mdcat OSC 8 hyperlink output: ESC]8;;url ESC\ visible_text ESC]8;; ESC\
        let line = "Visit \x1b]8;;https://example.com\x1b\\\x1b[34mhttps://example.com\x1b[0m\x1b]8;;\x1b\\ for info";
        let lines = wrap_line(line, 80);
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
        let lines = wrap_line(link, 5);
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
        let lines = wrap_line(line, 80);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], line);
    }

    #[test]
    fn test_wrap_line_charset_designation() {
        // Charset designation sequences: ESC ( B (ASCII), ESC ) 0 (DEC Special Graphics),
        // ESC * A, ESC + C, ESC % @ etc. — should all be skipped without consuming
        // visible text.
        let line = "\x1b(Bhello \x1b)0world\x1b*A!\x1b+C\x1b%@";
        let lines = wrap_line(line, 80);
        // Printable: "hello world!" = 12 chars, fits in 80 columns
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], line);
    }

    #[test]
    fn test_wrap_line_charset_designation_wrap() {
        // Ensure charset designation sequences don't affect wrap width calculation.
        let line = "\x1b(Babcde\x1b)0fghij";
        // "abcdefghij" = 10 printable chars; at width 5 it must wrap.
        let lines = wrap_line(line, 5);
        assert_eq!(lines.len(), 2);
    }
}
