use std::collections::VecDeque;
use std::str::from_utf8;
use log::error;

pub struct OutputBuffer {
    buffer: Vec<u8>,
    pub lines: VecDeque<String>,
    pub prompt: String,
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024),
            lines: VecDeque::with_capacity(1024),
            prompt: String::new(),
        }
    }

    pub fn buffer_to_prompt(&mut self) {
        if !self.buffer.is_empty() {
            self.prompt = if let Ok(line) = from_utf8(&self.buffer) {
                line.to_string()
            } else {
                error!("Unparsable line: {:?}", self.buffer);
                format!("Unparsable line: {:?}", self.buffer)
            };
            self.buffer.clear();
        }
    }

    pub fn receive(&mut self, data: &[u8]) -> Vec<String> {
        self.buffer.append(&mut Vec::from(data));

        let mut last_cut: usize = 0;
        let mut new_lines = vec![];
        for (i, bytes) in self.buffer.windows(2).enumerate() {
            if bytes == b"\r\n" {
                if i == 0 {
                    last_cut = 2
                } else {
                    let line = if let Ok(line) = from_utf8(&self.buffer[last_cut..i]) {
                        line.to_string()
                    } else {
                        error!("Unparsable line: {:?}", &self.buffer[last_cut..i]);
                        format!("Unparsable line: {:?}", &self.buffer[last_cut..i])
                    };
                    new_lines.push(line);
                    last_cut = i + 2;
                }
            }
        }
        if last_cut > 0 {
            self.buffer.drain(0..last_cut);
            self.lines.append(&mut VecDeque::from(new_lines.clone()));
        }
        new_lines
    }
}
