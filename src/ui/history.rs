use crate::model::Regex;

pub struct History {
    pub inner: Vec<String>,
    pub capacity: usize,
    pub drain_length: usize,
}

impl History {
    pub fn new() -> Self {
        let drain_length: usize = 1024;
        let capacity: usize = 32 * drain_length;
        Self {
            inner: Vec::with_capacity(capacity),
            capacity,
            drain_length,
        }
    }

    pub fn drain(&mut self) {
        if self.inner.len() >= self.capacity {
            self.inner.drain(0..self.drain_length);
        }
    }

    pub fn append(&mut self, line: &str) {
        if !line.trim().is_empty() {
            for line in line.lines() {
                self.inner.push(String::from(line));
            }
        } else {
            self.inner.push("".to_string());
        }
        self.drain();
    }

    pub fn remove_last(&mut self) -> Option<String> {
        self.inner.pop()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn find_forward(&self, pattern: &Regex, pos: usize) -> Option<usize> {
        self.inner[pos..]
            .iter()
            .position(|l| pattern.is_match(l))
            .map(|index| pos + index)
    }

    pub fn find_backward(&self, pattern: &Regex, pos: usize) -> Option<usize> {
        self.inner[..pos]
            .iter()
            .rev()
            .position(|l| pattern.is_match(l))
            .map(|index| pos - index - 1)
    }
}
