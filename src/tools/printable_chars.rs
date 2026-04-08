extern crate vte;

use std::str::{CharIndices, Chars};
use unicode_width::UnicodeWidthChar;
use vte::{Parser, Perform};

pub(crate) trait PrintableCharsIterator<'a> {
    fn printable_chars(&self) -> PrintableChars<'a>;
    fn printable_char_indices(&self) -> PrintableCharIndices<'a>;
    /// Returns the display width of the string (excluding ANSI escape sequences)
    fn display_width(&self) -> usize;
    /// Returns the byte index at which the display width reaches `target_width`,
    /// along with the actual display width at that point.
    /// If target_width is not reached, returns the end of string and total width.
    fn byte_index_at_display_width(&self, target_width: usize) -> (usize, usize);
}

impl<'a> PrintableCharsIterator<'a> for &'a str {
    fn printable_chars(&self) -> PrintableChars<'a> {
        PrintableChars::new(self)
    }

    fn printable_char_indices(&self) -> PrintableCharIndices<'a> {
        PrintableCharIndices::new(self)
    }

    fn display_width(&self) -> usize {
        self.printable_chars()
            .map(|c: char| c.width().unwrap_or(0))
            .sum()
    }

    fn byte_index_at_display_width(&self, target_width: usize) -> (usize, usize) {
        let mut width = 0;
        for (i, c) in self.printable_char_indices() {
            let char_width = c.width().unwrap_or(0);
            if width + char_width > target_width {
                return (i, width);
            }
            width += char_width;
        }
        (self.len(), width)
    }
}

struct Performer {
    c: Option<char>,
}

impl Performer {
    fn new() -> Self {
        Performer { c: None }
    }
}

impl Perform for Performer {
    fn print(&mut self, c: char) {
        self.c = Some(c)
    }
}

#[must_use = "iterators are lazy and do nothing unless consumed"]
pub(crate) struct PrintableChars<'a> {
    iter: Chars<'a>,
    parser: Parser,
    performer: Performer,
}

impl<'a> PrintableChars<'a> {
    fn new(s: &'a str) -> Self {
        PrintableChars {
            iter: s.chars(),
            parser: Parser::new(),
            performer: Performer::new(),
        }
    }
}

impl<'a> Iterator for PrintableChars<'a> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        let mut next = self.iter.next();
        let mut buf = [0u8; 4];

        while let Some(c) = next {
            let bytes = c.encode_utf8(&mut buf).as_bytes();
            self.parser.advance(&mut self.performer, bytes);
            if let Some(pc) = self.performer.c.take() {
                return Some(pc);
            } else {
                next = self.iter.next();
            }
        }

        None
    }
}

#[must_use = "iterators are lazy and do nothing unless consumed"]
pub(crate) struct PrintableCharIndices<'a> {
    iter: CharIndices<'a>,
    parser: Parser,
    performer: Performer,
}

impl<'a> PrintableCharIndices<'a> {
    fn new(s: &'a str) -> Self {
        PrintableCharIndices {
            iter: s.char_indices(),
            parser: Parser::new(),
            performer: Performer::new(),
        }
    }
}

impl<'a> Iterator for PrintableCharIndices<'a> {
    type Item = (usize, char);

    #[inline]
    fn next(&mut self) -> Option<(usize, char)> {
        let mut next = self.iter.next();
        let mut buf = [0u8; 4];

        while let Some((offset, c)) = next {
            let bytes = c.encode_utf8(&mut buf).as_bytes();
            self.parser.advance(&mut self.performer, bytes);
            if let Some(c) = self.performer.c.take() {
                return Some((offset, c));
            } else {
                next = self.iter.next();
            }
        }

        None
    }
}

#[cfg(test)]
mod test_printable_chars {
    use crate::tools::printable_chars::PrintableCharsIterator;

    const ANSI_RED: &str = "\x1b[30m";
    const ANSI_OFF: &str = "\x1b[0m";

    #[test]
    fn test_printable_chars() {
        let ansi_str = format!("Oh, {}hello{} there!", ANSI_RED, ANSI_OFF);
        let printable_str = ansi_str.as_str().printable_chars().collect::<String>();
        assert_ne!(ansi_str, printable_str);
        assert_eq!(printable_str, "Oh, hello there!".to_string())
    }

    #[test]
    fn test_printable_char_indices() {
        let ansi_str = format!("Oh, {}hello{} !", ANSI_RED, ANSI_OFF);
        let printable_indices = ansi_str
            .as_str()
            .printable_char_indices()
            .collect::<Vec<(usize, char)>>();
        let expected = vec![
            (0, 'O'),
            (1, 'h'),
            (2, ','),
            (3, ' '),
            (9, 'h'), // NB: indices 4,5,6,7,8 are skipped for ANSI_RED.
            (10, 'e'),
            (11, 'l'),
            (12, 'l'),
            (13, 'o'),
            (18, ' '), // NB: indices 14,15,16,17 skipped for ANSI_OFF.
            (19, '!'),
        ];
        assert_eq!(printable_indices, expected)
    }

    #[test]
    fn test_display_width() {
        // ASCII characters are 1 column wide
        assert_eq!("hello".display_width(), 5);

        // Chinese characters are 2 columns wide
        assert_eq!("中文".display_width(), 4);

        // Mixed content
        assert_eq!("中文test".display_width(), 8); // 2+2+1+1+1+1

        // ANSI escape sequences should not count
        let ansi_str = format!("{}hello{}", ANSI_RED, ANSI_OFF);
        assert_eq!(ansi_str.as_str().display_width(), 5);

        // Wide chars with ANSI
        let ansi_wide = format!("{}中文{}", ANSI_RED, ANSI_OFF);
        assert_eq!(ansi_wide.as_str().display_width(), 4);
    }

    #[test]
    fn test_byte_index_at_display_width() {
        // ASCII: each char is 1 byte and 1 column
        let (idx, width) = "hello".byte_index_at_display_width(3);
        assert_eq!(idx, 3);
        assert_eq!(width, 3);

        // Chinese: each char is 3 bytes and 2 columns
        // "中文test" = 中(3 bytes, 2 cols) + 文(3 bytes, 2 cols) + test(4 bytes, 4 cols)
        let input = "中文test";

        // At display width 4 (after both Chinese chars)
        let (idx, width) = input.byte_index_at_display_width(4);
        assert_eq!(idx, 6); // 3+3 bytes for 中文
        assert_eq!(width, 4);

        // At display width 3 (in the middle of second Chinese char)
        let (idx, width) = input.byte_index_at_display_width(3);
        assert_eq!(idx, 3); // Only 中 fits (3 bytes, 2 cols)
        assert_eq!(width, 2);

        // At display width 6 (after 中文te)
        let (idx, width) = input.byte_index_at_display_width(6);
        assert_eq!(idx, 8); // 6 bytes for 中文 + 2 bytes for "te"
        assert_eq!(width, 6);
    }
}
