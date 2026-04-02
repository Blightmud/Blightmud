use log::error;
use std::fmt;
use strip_ansi_escapes::strip as strip_ansi;

pub trait ToLine {
    fn to_line(&self) -> Line;
    fn to_internal_line(&self) -> Line;
}

impl ToLine for str {
    fn to_line(&self) -> Line {
        Line::from(self)
    }

    fn to_internal_line(&self) -> Line {
        let mut line = self.to_line();
        line.tag.key = "internal".to_string();
        line
    }
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub symbol: char,
    pub key: String,
    pub color: String,
}

impl Default for Tag {
    fn default() -> Self {
        Self {
            symbol: '┃',
            key: String::new(),
            color: String::new(),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Flags {
    pub gag: bool,
    pub skip_log: bool,
    pub replace_last: bool,
    pub prompt: bool,
    pub bypass_script: bool,
    pub matched: bool,
    pub tts_gag: bool,
    pub tts_interrupt: bool,
    pub separate_receives: bool,
    pub source: Option<String>,
    /// Set when the line contained screen-clearing escape sequences (ED sequences)
    /// that were filtered out. The UI should clear its output area when this is set.
    pub screen_clear: bool,
}

#[derive(Debug, Clone)]
pub struct Line {
    content: String,
    clean_content: String,
    clean_utf8: bool,
    pub tag: Tag,
    pub flags: Flags,
}

impl Eq for Line {}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content && self.clean_content == other.clean_content
    }
}

impl Line {
    pub fn from_codec(raw: &[u8], codec: Option<&'static encoding_rs::Encoding>) -> Self {
        if let Some(codec) = codec {
            let (line, _real_encoding, _is_sucess) = codec.decode(raw);

            line.as_bytes().into()
        } else {
            raw.into()
        }
    }
}

/// Filters out screen-clearing ED (Erase in Display) escape sequences from a string.
/// Returns (filtered_content, had_screen_clear).
/// ED sequences: ESC[J, ESC[0J, ESC[1J, ESC[2J, ESC[3J
fn filter_screen_clear_sequences(s: &str) -> (String, bool) {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut had_screen_clear = false;

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Check for CSI sequence (ESC[)
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['

                // Collect any digits
                let mut param = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() {
                        param.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // Check for 'J' (ED - Erase in Display)
                if chars.peek() == Some(&'J') {
                    chars.next(); // consume 'J'
                                  // Valid ED params are empty (defaults to 0), 0, 1, 2, or 3
                    if param == "1" || param == "2" || param == "3" {
                        had_screen_clear = true;
                        // Don't add this sequence to result - it's filtered out
                        continue;
                    } else if param.is_empty() || param == "0" {
                        // We are not interested in 'empty' and 0 since they clear from line to
                        // bottom of screen which isn't really applicable for Blightmud, so these are
                        // just filtered.
                        continue;
                    } else {
                        // Unknown ED param, preserve the sequence
                        result.push('\x1b');
                        result.push('[');
                        result.push_str(&param);
                        result.push('J');
                    }
                } else {
                    // Not an ED sequence, preserve what we consumed
                    result.push('\x1b');
                    result.push('[');
                    result.push_str(&param);
                }
            } else {
                // Not a CSI sequence, just an ESC
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    (result, had_screen_clear)
}

fn get_content_from(line: &str) -> (String, String, bool, bool) {
    let mut clean_utf8 = true;
    let trimmed = line.trim_end().to_string();

    // Filter out screen-clearing sequences before storing content
    let (content, screen_clear) = filter_screen_clear_sequences(&trimmed);

    let bytes = strip_ansi(&content);
    let clean_content = if let Ok(clean) = String::from_utf8(bytes.clone()) {
        clean
    } else {
        error!("[Line]: Unparsable &str : {:?}", line);
        clean_utf8 = false;
        String::from_utf8_lossy(&bytes).to_mut().clone()
    };
    let clean_content = clean_content.replace('\r', "");
    (content, clean_content, clean_utf8, screen_clear)
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

impl From<&Line> for Line {
    fn from(line: &Line) -> Self {
        Self {
            content: line.content.clone(),
            clean_content: line.clean_content.clone(),
            clean_utf8: line.clean_utf8,
            tag: line.tag.clone(),
            flags: line.flags.clone(),
        }
    }
}

impl From<&str> for Line {
    fn from(line: &str) -> Self {
        let (content, clean_content, clean_utf8, screen_clear) = get_content_from(line);
        Self {
            content,
            clean_content,
            clean_utf8,
            tag: Tag::default(),
            flags: Flags {
                screen_clear,
                ..Flags::default()
            },
        }
    }
}

impl From<String> for Line {
    fn from(line: String) -> Self {
        let (content, clean_content, clean_utf8, screen_clear) = get_content_from(&line);
        Self {
            content,
            clean_content,
            clean_utf8,
            tag: Tag::default(),
            flags: Flags {
                screen_clear,
                ..Flags::default()
            },
        }
    }
}

impl From<&String> for Line {
    fn from(line: &String) -> Self {
        let (content, clean_content, clean_utf8, screen_clear) = get_content_from(line);
        Self {
            content,
            clean_content,
            clean_utf8,
            tag: Tag::default(),
            flags: Flags {
                screen_clear,
                ..Flags::default()
            },
        }
    }
}

impl From<&[u8]> for Line {
    fn from(line: &[u8]) -> Self {
        let line = if let Ok(line) = String::from_utf8(line.to_vec()) {
            line
        } else {
            error!("[Line]: Unparsable bytes : {:?}", line);
            String::from_utf8_lossy(line).to_mut().clone()
        };

        let (content, clean_content, clean_utf8, screen_clear) = get_content_from(&line);
        Self {
            content,
            clean_content,
            clean_utf8,
            tag: Tag::default(),
            flags: Flags {
                screen_clear,
                ..Flags::default()
            },
        }
    }
}

impl From<&Vec<u8>> for Line {
    fn from(line: &Vec<u8>) -> Self {
        let mut clean_utf8 = true;
        let line = if let Ok(line) = String::from_utf8(line.clone()) {
            line
        } else {
            clean_utf8 = false;
            String::from_utf8_lossy(line).to_mut().clone()
        };

        let (content, clean_content, _, screen_clear) = get_content_from(&line);
        Self {
            content,
            clean_content,
            clean_utf8,
            tag: Tag::default(),
            flags: Flags {
                screen_clear,
                ..Flags::default()
            },
        }
    }
}

#[allow(dead_code)]
impl Line {
    pub fn set_content(&mut self, line: &str) {
        let (content, clean_content, clean_utf8, screen_clear) = get_content_from(line);
        self.content = content;
        self.clean_content = clean_content;
        self.clean_utf8 = clean_utf8;
        self.flags.screen_clear = screen_clear;
    }

    pub fn print_line(&self) -> Option<&str> {
        if !self.flags.gag {
            Some(self.content.as_str())
        } else {
            None
        }
    }

    pub fn tagged_line(&self) -> Option<String> {
        self.print_line().map(|content| {
            if self.tag.color.is_empty() {
                format!("  {}", content)
            } else {
                format!("{}{} \x1b[0m{}", self.tag.color, self.tag.symbol, content)
            }
        })
    }

    pub fn is_utf8(&self) -> bool {
        self.clean_utf8
    }

    pub fn log_line(&self) -> Option<&str> {
        if self.flags.skip_log || (self.flags.prompt && self.content.is_empty()) {
            None
        } else {
            Some(self.clean_content.as_str())
        }
    }

    pub fn line(&self) -> &str {
        &self.content
    }

    pub fn clean_line(&self) -> &str {
        &self.clean_content
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.clean_content.clear();
    }

    pub fn lines(&'_ self) -> std::str::Lines<'_> {
        self.content.lines()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn replace_with(&mut self, other: &Line) {
        self.flags = other.flags.clone();
    }
}

#[cfg(test)]
mod test_line {

    use super::Line;

    #[test]
    fn test_gag_flag() {
        let mut line = Line::from("testline");
        assert_eq!(line.log_line(), Some("testline"));
        assert_eq!(line.print_line(), Some("testline"));
        line.flags.gag = true;
        assert_eq!(line.log_line(), Some("testline"));
        assert_eq!(line.print_line(), None);
    }

    #[test]
    fn test_skip_log_flag() {
        let mut line = Line::from("testline");
        assert_eq!(line.log_line(), Some("testline"));
        assert_eq!(line.print_line(), Some("testline"));
        line.flags.skip_log = true;
        assert_eq!(line.print_line(), Some("testline"));
        assert_eq!(line.log_line(), None);
    }

    #[test]
    fn test_carriage_return_strip() {
        let line = Line::from("\r\rtestline");
        assert_eq!(line.line(), "\r\rtestline");
        assert_eq!(line.clean_line(), "testline");
    }

    #[test]
    fn test_bad_utf8() {
        let line = Line::from("a good line");
        assert_eq!(line.is_utf8(), true);
        let line = Line::from(&vec![
            0xF0, 0xA4, 0xAD, 0xF0, 0xA4, 0xAD, 0xA2, 0xF0, 0xA4, 0xAD, 0xA2, 0xF0, 0xA4, 0xAD,
            0xA2, 0xF0, 0xA4, 0xAD, 0xA2, 0xF0, 0xA4, 0xAD,
        ]);
        assert_eq!(line.is_utf8(), false);
    }

    #[test]
    fn test_set_content() {
        let mut line = Line::from("test");
        assert_eq!(line.line(), "test");
        line.set_content("\x1b[32mbatman\x1b[0m");
        assert_eq!(line.clean_line(), "batman");
        assert_eq!(line.line(), "\u{1b}[32mbatman\u{1b}[0m");
    }

    #[test]
    fn test_clear() {
        let mut line = Line::from("test");
        assert_eq!(line.line(), "test");
        assert_eq!(line.clean_line(), "test");
        line.clear();
        assert_eq!(line.line(), "");
        assert_eq!(line.clean_line(), "");
    }

    #[test]
    fn test_line_from_line() {
        let mut line = Line::from("test");
        let other = Line::from("test");
        line.flags.skip_log = true;
        line.flags.prompt = true;
        let clone = Line::from(&line);
        assert_eq!(clone, line);
        assert_eq!(clone.flags, line.flags);
        assert_eq!(other, line);
        assert_ne!(other.flags, line.flags);
    }

    #[test]
    fn test_from_string() {
        let line = Line::from("test".to_string());
        assert_eq!(line.line(), "test");
        let line = Line::from(&"test".to_string());
        assert_eq!(line.line(), "test");
    }

    #[test]
    fn test_display() {
        let line = Line::from("test");
        assert_eq!(format!("{}", line), "test".to_string());
        let line = Line::from("\x1b[32mbatman\x1b[0m");
        assert_eq!(format!("{}", line), "\u{1b}[32mbatman\u{1b}[0m".to_string());
    }

    #[test]
    fn test_lines() {
        let line = Line::from("test1\r\ntest2\r\ntest3");
        let mut it = line.lines();
        assert_eq!(it.next(), Some("test1"));
        assert_eq!(it.next(), Some("test2"));
        assert_eq!(it.next(), Some("test3"));
    }

    #[test]
    fn test_screen_clear_filter_esc_j() {
        // ESC[J (default, same as ESC[0J)
        let line = Line::from("before\x1b[Jafter");
        assert_eq!(line.line(), "beforeafter");
        assert!(!line.flags.screen_clear);
    }

    #[test]
    fn test_screen_clear_filter_esc_0j() {
        // ESC[0J - clear from cursor to end of screen
        let line = Line::from("before\x1b[0Jafter");
        assert_eq!(line.line(), "beforeafter");
        assert!(!line.flags.screen_clear);
    }

    #[test]
    fn test_screen_clear_filter_esc_1j() {
        // ESC[1J - clear from cursor to beginning of screen
        let line = Line::from("before\x1b[1Jafter");
        assert_eq!(line.line(), "beforeafter");
        assert!(line.flags.screen_clear);
    }

    #[test]
    fn test_screen_clear_filter_esc_2j() {
        // ESC[2J - clear entire screen
        let line = Line::from("before\x1b[2Jafter");
        assert_eq!(line.line(), "beforeafter");
        assert!(line.flags.screen_clear);
    }

    #[test]
    fn test_screen_clear_filter_esc_3j() {
        // ESC[3J - clear entire screen and scrollback
        let line = Line::from("before\x1b[3Jafter");
        assert_eq!(line.line(), "beforeafter");
        assert!(line.flags.screen_clear);
    }

    #[test]
    fn test_screen_clear_filter_preserves_other_escapes() {
        // Color codes should be preserved
        let line = Line::from("\x1b[32mgreen\x1b[0m");
        assert_eq!(line.line(), "\x1b[32mgreen\x1b[0m");
        assert!(!line.flags.screen_clear);
    }

    #[test]
    fn test_screen_clear_filter_mixed() {
        // Screen clear mixed with color codes
        let line = Line::from("\x1b[32mgreen\x1b[2J\x1b[0mtext");
        assert_eq!(line.line(), "\x1b[32mgreen\x1b[0mtext");
        assert!(line.flags.screen_clear);
    }

    #[test]
    fn test_screen_clear_no_false_positives() {
        // ESC[K is erase in line, not erase in display - should not trigger screen_clear
        let line = Line::from("test\x1b[Kmore");
        assert_eq!(line.line(), "test\x1b[Kmore");
        assert!(!line.flags.screen_clear);
    }

    #[test]
    fn test_screen_clear_multiple() {
        // Multiple clear sequences
        let line = Line::from("\x1b[2J\x1b[J\x1b[1J");
        assert_eq!(line.line(), "");
        assert!(line.flags.screen_clear);
    }

    #[test]
    fn test_tagged_line_default() {
        let line = Line::from("hello");
        // Default tag has no color: render two spaces
        assert_eq!(line.tagged_line(), Some("  hello".to_string()));
    }

    #[test]
    fn test_tagged_line_with_color() {
        let mut line = Line::from("hello");
        line.tag.color = "\x1b[31m".to_string();
        assert_eq!(
            line.tagged_line(),
            Some("\x1b[31m┃ \x1b[0mhello".to_string())
        );
    }

    #[test]
    fn test_tagged_line_gagged() {
        let mut line = Line::from("hello");
        line.flags.gag = true;
        assert_eq!(line.tagged_line(), None);
    }
}
