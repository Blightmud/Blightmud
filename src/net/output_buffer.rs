use log::debug;

use crate::model::Line;

use super::{tcp_stream::BUFFER_SIZE, telnet::TelnetMode};

pub struct OutputBuffer {
    buffer: Vec<u8>,
    telnet_mode: TelnetMode,
    new_data: bool,
}

impl OutputBuffer {
    pub fn new(telnet_mode: &TelnetMode) -> Self {
        Self {
            buffer: Vec::with_capacity(BUFFER_SIZE),
            telnet_mode: telnet_mode.clone(),
            new_data: false,
        }
    }

    pub fn telnet_mode(&mut self, mode: &TelnetMode) {
        self.telnet_mode = mode.clone();
    }

    pub fn buffer_to_prompt(&mut self, consume_buffer: bool) -> Line {
        let mut prompt = if !self.buffer.is_empty() {
            Line::from(&self.buffer)
        } else {
            Line::from("")
        };
        prompt.flags.prompt = true;
        if consume_buffer {
            self.buffer.clear();
        }
        self.new_data = false;
        prompt
    }

    pub fn has_new_data(&self) -> bool {
        self.new_data
    }

    pub fn input_sent(&mut self) {
        debug!("Input sent. Mode: {:?}", self.telnet_mode);
        if self.telnet_mode == TelnetMode::UnterminatedPrompt {
            self.buffer.clear();
        }
    }

    pub fn receive(&mut self, data: &[u8]) -> Vec<Line> {
        let existing_buffer_len = self.buffer.len();
        self.new_data = true;

        self.buffer.append(&mut Vec::from(data));

        let cut_line =
            |lines: &mut Vec<Line>, i: usize, last_cut: usize, cut_len: usize| -> usize {
                if i == 0 {
                    lines.push(Line::from("".to_string()));
                    cut_len
                } else {
                    let mut line = Line::from(&self.buffer[last_cut..i]);
                    if self.telnet_mode == TelnetMode::UnterminatedPrompt && last_cut == 0 {
                        line.flags.separate_receives =
                            i > existing_buffer_len && existing_buffer_len > 0;
                    }
                    lines.push(line);
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

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.telnet_mode = TelnetMode::default();
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod output_buffer_tests {

    use super::OutputBuffer;
    use crate::{model::Line, net::TelnetMode};

    #[test]
    fn test_prompt_capture() {
        let mut buffer = OutputBuffer::new(&TelnetMode::default());
        assert!(buffer.is_empty());
        let lines = buffer.receive(b"Some misc output\r\nfollowed by some more output\r\nprompt");
        assert_eq!(lines.len(), 2);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.buffer_to_prompt(true), Line::from("prompt"));
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_line_capture() {
        let mut buffer = OutputBuffer::new(&TelnetMode::default());
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
        let mut buffer = OutputBuffer::new(&TelnetMode::default());
        let lines = buffer.receive(b"word1\r\nword2\r\nword3\r\nprompt");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("word1")));
        assert_eq!(iter.next(), Some(&Line::from("word2")));
        assert_eq!(iter.next(), Some(&Line::from("word3")));
        assert_eq!(buffer.buffer_to_prompt(true), Line::from("prompt"));
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_nr_line_parsing() {
        let mut buffer = OutputBuffer::new(&TelnetMode::default());
        let lines = buffer.receive(b"word1\n\rword2\n\rword3\n\rprompt");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("word1")));
        assert_eq!(iter.next(), Some(&Line::from("word2")));
        assert_eq!(iter.next(), Some(&Line::from("word3")));
        assert_eq!(buffer.buffer_to_prompt(true), Line::from("prompt"));
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_n_line_parsing() {
        let mut buffer = OutputBuffer::new(&TelnetMode::default());
        let lines = buffer.receive(b"word1\nword2\nword3\nprompt");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("word1")));
        assert_eq!(iter.next(), Some(&Line::from("word2")));
        assert_eq!(iter.next(), Some(&Line::from("word3")));
        assert_eq!(buffer.buffer_to_prompt(true), Line::from("prompt"));
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_carriage_return_removal() {
        let mut buffer = OutputBuffer::new(&TelnetMode::default());
        let lines = buffer.receive(b"word1\n\r\r\rword2\r\r\r\nword3\nprompt\r");
        let mut iter = lines.iter();
        assert_eq!(iter.next(), Some(&Line::from("word1")));
        assert_eq!(iter.next(), Some(&Line::from("\r\rword2")));
        assert_eq!(iter.next(), Some(&Line::from("word3")));
        assert_eq!(buffer.buffer_to_prompt(true), Line::from("prompt"));
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_clean_line_match() {
        let mut buffer = OutputBuffer::new(&TelnetMode::default());
        let lines = buffer.receive(b"\n\r   \rword1\n\r\r\rprompt\r");
        let mut iter = lines.iter();
        let _ = iter.next();
        let line = iter.next().unwrap();
        assert_eq!(line.line(), "   \rword1");
        assert_eq!(line.clean_line(), "   word1");
        assert_eq!(buffer.buffer_to_prompt(true), Line::from("\r\rprompt"));
    }

    #[test]
    fn test_separate_receives() {
        let mut buffer = OutputBuffer::new(&TelnetMode::UnterminatedPrompt);
        let first_lines = buffer.receive(b"line 1\nline 2\r\nline 3\n\rMaybe a prompt...");
        assert_eq!(buffer.buffer_to_prompt(false), Line::from("Maybe a prompt..."));
        let second_lines = buffer.receive(b" but no, it isn't a prompt\nline 4\r\nline 5\n\rDefinitely a prompt");
        assert_eq!(buffer.buffer_to_prompt(false), Line::from("Definitely a prompt"));

        let mut iter = first_lines.iter().chain(second_lines.iter());
        assert_eq!(iter.next(), Some(&Line::from("line 1")));
        assert_eq!(iter.next(), Some(&Line::from("line 2")));
        assert_eq!(iter.next(), Some(&Line::from("line 3")));
        assert_eq!(iter.next(), Some(&Line::from("Maybe a prompt... but no, it isn't a prompt")));
        assert_eq!(iter.next(), Some(&Line::from("line 4")));
        assert_eq!(iter.next(), Some(&Line::from("line 5")));
        assert_eq!(iter.next(), None);
    }
}
