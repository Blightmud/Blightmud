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

    pub fn remove_last_if_prefix(&mut self, line: &str) -> Option<String> {
        if let Some(prefix) = self.inner.last() {
            if line.starts_with(prefix) {
                self.inner.pop()
            } else {
                None
            }
        } else {
            None
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic() {
        let mut history = History::new();
        assert!(history.is_empty());
        history.append("test");
        assert!(!history.is_empty());
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_remove_last() {
        let mut history = History::new();
        history.append("a nice line");
        history.append("a complete line");
        history.append("a par");

        assert_eq!(history.len(), 3);
        history.remove_last_if_prefix("a fancy prompt");
        assert_eq!(history.len(), 3);
        history.remove_last_if_prefix("a partial line");
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn confirm_drain() {
        let mut history = History::new();
        for _ in 0..31 * 1024 {
            history.append("test");
        }
        assert_eq!(history.len(), 31 * 1024);
        for _ in 0..1024 {
            history.append("test");
        }
        assert_eq!(history.len(), 31 * 1024);
    }

    #[test]
    fn test_find() {
        let mut history = History::new();
        for i in 0..12000 {
            if i % 1000 == 0 {
                history.append("something");
            } else {
                history.append("nothing");
            }
        }
        let mut index = history.len();
        let mut goal = 11000;
        let pattern = Regex::new("^something$", None).unwrap();
        while index > 0 && goal > 0 {
            index = if let Some(i) = history.find_backward(&pattern, index) {
                i
            } else {
                0
            };
            assert_eq!(index, goal);
            goal -= 1000;
        }
        goal += 1000;
        while index < history.len() && goal <= 11000 {
            index = if let Some(i) = history.find_forward(&pattern, index) {
                i
            } else {
                0
            };
            assert_eq!(index, goal);
            goal += 1000;
            index += 1;
        }
    }
}
