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
    pub flags: Flags,
}

impl Eq for Line {}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}

fn get_content_from(line: &str) -> (String, String) {
    let content = line.trim_end().to_string();
    let clean_content = if let Ok(bytes) = strip_ansi(&content) {
        if let Ok(clean) = String::from_utf8(bytes.clone()) {
            clean
        } else {
            String::from_utf8_lossy(&bytes).to_mut().clone()
        }
    } else {
        "".to_string()
    };
    (content, clean_content)
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
            flags: line.flags.clone(),
        }
    }
}

impl From<&str> for Line {
    fn from(line: &str) -> Self {
        let (content, clean_content) = get_content_from(line);
        Self {
            content,
            clean_content,
            flags: Flags::default(),
        }
    }
}

impl From<String> for Line {
    fn from(line: String) -> Self {
        let (content, clean_content) = get_content_from(&line);
        Self {
            content,
            clean_content,
            flags: Flags::default(),
        }
    }
}

impl From<&String> for Line {
    fn from(line: &String) -> Self {
        let (content, clean_content) = get_content_from(&line);
        Self {
            content,
            clean_content,
            flags: Flags::default(),
        }
    }
}

impl From<&[u8]> for Line {
    fn from(line: &[u8]) -> Self {
        let line = if let Ok(line) = String::from_utf8(line.to_vec()) {
            line
        } else {
            String::from_utf8_lossy(line).to_mut().clone()
        };

        let (content, clean_content) = get_content_from(&line);
        Self {
            content,
            clean_content,
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

        let (content, clean_content) = get_content_from(&line);
        Self {
            content,
            clean_content,
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
}
