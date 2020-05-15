use log::error;

pub struct OutputBuffer {
    buffer: Vec<u8>,
    pub prompt: String,
}

fn read_string_from(buffer: &[u8]) -> String {
    if let Ok(string) = String::from_utf8(buffer.to_vec()) {
        string
    } else {
        error!("Unparsable bytes: {:?}", buffer);
        String::from_utf8_lossy(buffer).to_mut().clone()
    }
}

impl OutputBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024),
            prompt: String::new(),
        }
    }

    pub fn buffer_to_prompt(&mut self) {
        if !self.buffer.is_empty() {
            self.prompt = read_string_from(&self.buffer);
            self.buffer.clear();
        } else {
            self.prompt.clear();
        }
    }

    pub fn receive(&mut self, data: &[u8]) -> Vec<String> {
        self.buffer.append(&mut Vec::from(data));

        let mut last_cut: usize = 0;
        let mut lines: Vec<String> = vec![];
        for (i, bytes) in self.buffer.windows(2).enumerate() {
            if bytes == b"\r\n" {
                if i == 0 {
                    last_cut = 2
                } else {
                    let line: String = read_string_from(&self.buffer[last_cut..i]);
                    lines.push(line);
                    last_cut = i + 2;
                }
            }
        }
        if last_cut > 0 {
            if last_cut < self.buffer.len() {
                self.buffer.drain(0..last_cut);
            } else {
                self.buffer.clear();
            }
        }
        lines
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.prompt.clear();
    }
}
