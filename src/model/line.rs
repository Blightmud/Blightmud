use log::error;
use std::fmt;
use strip_ansi_escapes::strip as strip_ansi;

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
}

#[derive(Debug, Clone)]
pub struct Line {
    content: String,
    clean_content: String,
    clean_utf8: bool,
    pub flags: Flags,
}

impl Eq for Line {}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content && self.clean_content == other.clean_content
    }
}

fn get_content_from(line: &str) -> (String, String, bool) {
    let mut clean_utf8 = true;
    let content = line.trim_end().to_string();
    let clean_content = if let Ok(bytes) = strip_ansi(&content) {
        if let Ok(clean) = String::from_utf8(bytes.clone()) {
            clean
        } else {
            error!("[Line]: Unparsable &str : {:?}", line);
            clean_utf8 = false;
            String::from_utf8_lossy(&bytes).to_mut().clone()
        }
    } else {
        "".to_string()
    };
    let clean_content = clean_content.replace('\r', "");
    (content, clean_content, clean_utf8)
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
            flags: line.flags.clone(),
        }
    }
}

impl From<&str> for Line {
    fn from(line: &str) -> Self {
        let (content, clean_content, clean_utf8) = get_content_from(line);
        Self {
            content,
            clean_content,
            clean_utf8,
            flags: Flags::default(),
        }
    }
}

impl From<String> for Line {
    fn from(line: String) -> Self {
        let (content, clean_content, clean_utf8) = get_content_from(&line);
        Self {
            content,
            clean_content,
            clean_utf8,
            flags: Flags::default(),
        }
    }
}

impl From<&String> for Line {
    fn from(line: &String) -> Self {
        let (content, clean_content, clean_utf8) = get_content_from(line);
        Self {
            content,
            clean_content,
            clean_utf8,
            flags: Flags::default(),
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

        let (content, clean_content, clean_utf8) = get_content_from(&line);
        Self {
            content,
            clean_content,
            clean_utf8,
            flags: Flags::default(),
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

        let (content, clean_content, _) = get_content_from(&line);
        Self {
            content,
            clean_content,
            clean_utf8,
            flags: Flags::default(),
        }
    }
}

#[allow(dead_code)]
impl Line {
    pub fn set_content(&mut self, line: &str) {
        let (content, clean_content, clean_utf8) = get_content_from(line);
        self.content = content;
        self.clean_content = clean_content;
        self.clean_utf8 = clean_utf8;
    }

    pub fn print_line(&self) -> Option<&str> {
        if !self.flags.gag {
            Some(self.content.as_str())
        } else {
            None
        }
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

    pub fn lines(&self) -> std::str::Lines {
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
}
