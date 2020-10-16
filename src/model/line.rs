use log::error;
use std::fmt;
use strip_ansi_escapes::strip as strip_ansi;

#[derive(Debug, Default, Clone)]
pub struct Flags {
    pub gag: bool,
    pub skip_log: bool,
    pub replace_last: bool,
    pub prompt: bool,
    pub bypass_script: bool,
    pub matched: bool,
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
    let clean_content = clean_content.replace("\r", "");
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
        let (content, clean_content, clean_utf8) = get_content_from(&line);
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
        let mut clean_utf8 = true;
        let line = if let Ok(line) = String::from_utf8(line.to_vec()) {
            line
        } else {
            clean_utf8 = false;
            error!("[Line]: Unparsable bytes : {:?}", line);
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

impl From<&Vec<u8>> for Line {
    fn from(line: &Vec<u8>) -> Self {
        let line = if let Ok(line) = String::from_utf8(line.clone()) {
            line
        } else {
            String::from_utf8_lossy(&line).to_mut().clone()
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

#[allow(dead_code)]
impl Line {
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
        if !self.flags.skip_log {
            Some(self.clean_content.as_str())
        } else {
            None
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
}
