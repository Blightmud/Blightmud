extern crate vte;

use std::str::{CharIndices, Chars};
use vte::{Parser, Perform};

pub(crate) trait PrintableCharsIterator<'a> {
    fn printable_chars(&self) -> PrintableChars<'a>;
    fn printable_char_indices(&self) -> PrintableCharIndices<'a>;
}

impl<'a> PrintableCharsIterator<'a> for &'a str {
    fn printable_chars(&self) -> PrintableChars<'a> {
        PrintableChars::new(self)
    }

    fn printable_char_indices(&self) -> PrintableCharIndices<'a> {
        PrintableCharIndices::new(self)
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

        while let Some(c) = next {
            self.parser.advance(&mut self.performer, &[c as u8]);
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

        while let Some((offset, c)) = next {
            self.parser.advance(&mut self.performer, &[c as u8]);
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
    use crate::ui::printable_chars::PrintableCharsIterator;

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
}
