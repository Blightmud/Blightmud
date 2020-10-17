use crate::model::Line;

use super::tcp_stream::BUFFER_SIZE;

pub struct OutputBuffer {
    buffer: Vec<u8>,
    pub prompt: Line,
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(BUFFER_SIZE),
            prompt: Line::from(""),
        }
    }

    pub fn buffer_to_prompt(&mut self, consume_buffer: bool) {
        if !self.buffer.is_empty() {
            self.prompt = Line::from(&self.buffer);
            if consume_buffer {
                self.buffer.clear();
            }
        } else {
            self.prompt.clear();
        }
    }

    pub fn receive(&mut self, data: &[u8]) -> Vec<Line> {
        self.buffer.append(&mut Vec::from(data));

        let cut_line =
            |lines: &mut Vec<Line>, i: usize, last_cut: usize, cut_len: usize| -> usize {
                if i == 0 {
                    lines.push(Line::from("".to_string()));
                    cut_len
                } else {
                    lines.push(Line::from(&self.buffer[last_cut..i]));
                    i + cut_len
                }
            };

        let mut last_cut: usize = 0;
        let mut lines: Vec<Line> = vec![];
        for (i, bytes) in self.buffer.windows(2).enumerate() {
            if i >= last_cut && (bytes == b"\r\n" || bytes == b"\n\r") {
                last_cut = cut_line(&mut lines, i, last_cut, 2);
            } else if i >= last_cut && bytes[0] == b'\n' {
                last_cut = cut_line(&mut lines, i, last_cut, 1);
            }
        }
        if last_cut > 0 {
            self.buffer.drain(0..last_cut);
        }
        lines
    }

    pub fn flush(&mut self) {
        self.buffer.clear();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.prompt.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod output_buffer_tests {

    use super::OutputBuffer;
    use crate::model::Line;

    #[test]
    fn test_prompt_capture() {
        let mut buffer = OutputBuffer::new();
        assert!(buffer.is_empty());
        let lines = buffer.receive(b"Some misc output\r\nfollowed by some more output\r\nprompt");
        assert_eq!(lines.len(), 2);
        assert!(!buffer.is_empty());
        buffer.buffer_to_prompt(true);
        assert_eq!(buffer.prompt, Line::from("prompt"));
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_line_capture() {
        let mut buffer = OutputBuffer::new();
        let lines = buffer.receive(b"Some misc output\r\nfollowed by some more output\r\nprompt");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("Some misc output")));
        assert_eq!(
            iter.next(),
            Some(&Line::from("followed by some more output"))
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_rn_line_parsing() {
        let mut buffer = OutputBuffer::new();
        let lines = buffer.receive(b"word1\r\nword2\r\nword3\r\nprompt");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("word1")));
        assert_eq!(iter.next(), Some(&Line::from("word2")));
        assert_eq!(iter.next(), Some(&Line::from("word3")));
        buffer.buffer_to_prompt(true);
        assert_eq!(buffer.prompt, Line::from("prompt"));
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_nr_line_parsing() {
        let mut buffer = OutputBuffer::new();
        let lines = buffer.receive(b"word1\n\rword2\n\rword3\n\rprompt");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("word1")));
        assert_eq!(iter.next(), Some(&Line::from("word2")));
        assert_eq!(iter.next(), Some(&Line::from("word3")));
        buffer.buffer_to_prompt(true);
        assert_eq!(buffer.prompt, Line::from("prompt"));
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_n_line_parsing() {
        let mut buffer = OutputBuffer::new();
        let lines = buffer.receive(b"word1\nword2\nword3\nprompt");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("word1")));
        assert_eq!(iter.next(), Some(&Line::from("word2")));
        assert_eq!(iter.next(), Some(&Line::from("word3")));
        buffer.buffer_to_prompt(true);
        assert_eq!(buffer.prompt, Line::from("prompt"));
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_carriage_return_removal() {
        let mut buffer = OutputBuffer::new();
        let lines = buffer.receive(b"word1\n\r\r\rword2\r\r\r\nword3\nprompt\r");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("word1")));
        assert_eq!(iter.next(), Some(&Line::from("\r\rword2")));
        assert_eq!(iter.next(), Some(&Line::from("word3")));
        buffer.buffer_to_prompt(true);
        assert_eq!(buffer.prompt, Line::from("prompt"));
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_clean_line_match() {
        let mut buffer = OutputBuffer::new();
        let lines = buffer.receive(b"\n\r   \rword1\n\r\r\rprompt\r");
        let mut iter = lines.iter();
        let _ = iter.next();
        let line = iter.next().unwrap();
        assert_eq!(line.line(), "   \rword1");
        assert_eq!(line.clean_line(), "   word1");
        buffer.buffer_to_prompt(true);
        assert_eq!(buffer.prompt, Line::from("\r\rprompt"));
    }
}
