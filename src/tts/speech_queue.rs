use std::collections::VecDeque;

pub struct SpeechQueue {
    capacity: usize,
    index: usize,
    scan_index: usize,
    queue: VecDeque<String>,
}

impl SpeechQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            index: 0,
            scan_index: 0,
            queue: VecDeque::default(),
        }
    }

    pub fn push(&mut self, msg: String, force: bool) -> Option<String> {
        self.queue.push_back(msg.clone());
        let speak_next = self.index == self.queue.len() - 1;
        let scanning = self.scan_index < self.queue.len() - 1;
        if !scanning {
            self.scan_index += 1;
        }
        while self.queue.len() > self.capacity {
            self.queue.pop_front();
            self.index = (self.index as i32 - 1).max(0) as usize;
            if scanning {
                self.scan_index = (self.scan_index as i32 - 1).max(0) as usize;
            }
        }
        if force || speak_next {
            if msg.is_empty() {
                self.index = self.queue.len();
                None
            } else {
                self.index = self.queue.len() - 1;
                Some(msg)
            }
        } else {
            None
        }
    }

    pub fn scan_back(&mut self, step: usize) -> Option<String> {
        self.scan_index = (self.scan_index as i32 - step as i32).max(0) as usize;
        Some(self.queue[self.scan_index].clone())
    }

    pub fn scan_forward(&mut self, step: usize) -> Option<String> {
        self.scan_index = (self.scan_index + step).min(self.queue.len());
        if self.scan_index < self.queue.len() {
            Some(self.queue[self.scan_index].clone())
        } else {
            None
        }
    }

    pub fn next(&mut self, step: usize) -> Option<String> {
        self.index = (self.index + step).min(self.queue.len());

        while self.index < self.queue.len() && self.queue[self.index].is_empty() {
            self.index += 1;
        }

        if self.index < self.queue.len() {
            Some(self.queue[self.index].clone())
        } else {
            None
        }
    }

    pub fn current(&mut self) -> Option<String> {
        if self.index < self.queue.len() {
            Some(self.queue[self.index].clone())
        } else {
            None
        }
    }

    pub fn prev(&mut self, step: usize) -> Option<String> {
        self.index = (self.index as i32 - step as i32).max(0) as usize;

        while self.index > 0 && self.queue[self.index].is_empty() {
            self.index -= 1;
        }

        if self.queue[self.index].is_empty() {
            None
        } else {
            Some(self.queue[self.index].clone())
        }
    }

    pub fn flush(&mut self) {
        self.index = self.queue.len();
        self.scan_index = self.queue.len();
    }
}

#[cfg(test)]
mod test_speech_queue {
    use super::SpeechQueue;

    #[test]
    fn test_push() {
        let mut q = SpeechQueue::new(100);
        assert_eq!(
            q.push("test1".to_string(), false),
            Some("test1".to_string())
        );
        assert_eq!(q.index, 0);
        assert_eq!(q.push("test2".to_string(), false), None);
        assert_eq!(q.push("test3".to_string(), false), None);
        assert_eq!(q.push("test4".to_string(), false), None);
        assert_eq!(q.push("test5".to_string(), false), None);
        assert_eq!(q.push("test6".to_string(), false), None);
        assert_eq!(q.next(1), Some("test2".to_string()));
        assert_eq!(q.index, 1);
        assert_eq!(q.next(1), Some("test3".to_string()));
        assert_eq!(q.index, 2);
        assert_eq!(q.next(2), Some("test5".to_string()));
        assert_eq!(q.index, 4);
        assert_eq!(q.next(10), None);
    }

    #[test]
    fn test_prev_next() {
        let mut q = SpeechQueue::new(100);
        q.push("test1".to_string(), false);
        q.push("test2".to_string(), false);
        q.push("test3".to_string(), false);
        q.push("test4".to_string(), false);
        assert_eq!(q.index, 0);
        q.flush();
        assert_eq!(q.index, 4);
        assert_eq!(q.prev(100), Some("test1".to_string()));
        assert_eq!(q.next(100), None);
    }

    #[test]
    fn test_current() {
        let mut q = SpeechQueue::new(100);
        q.push("test1".to_string(), false);
        q.push("test2".to_string(), false);
        q.push("test3".to_string(), false);
        q.push("test4".to_string(), false);
        assert_eq!(q.index, 0);
        assert_eq!(q.current(), Some("test1".to_string()));
        assert_eq!(q.next(1), Some("test2".to_string()));
        assert_eq!(q.current(), Some("test2".to_string()));
        assert_eq!(q.next(10), None);
        assert_eq!(q.current(), None);
    }

    #[test]
    fn test_flush() {
        let mut q = SpeechQueue::new(100);
        q.push("test".to_string(), false);
        q.push("test".to_string(), false);
        q.push("test".to_string(), false);
        q.push("test".to_string(), false);
        assert_eq!(q.index, 0);
        q.flush();
        assert_eq!(q.index, 4);
    }

    #[test]
    fn test_force() {
        let mut q = SpeechQueue::new(100);
        q.push("test".to_string(), false);
        q.push("test".to_string(), false);
        q.push("test".to_string(), false);
        q.push("test".to_string(), false);
        assert_eq!(q.index, 0);
        q.push("new_line".to_string(), true);
        assert_eq!(q.index, q.queue.len() - 1);
        assert_eq!(q.current(), Some("new_line".to_string()));
    }

    #[test]
    fn test_overflow() {
        let mut q = SpeechQueue::new(10);
        for i in 0..10 {
            q.push(format!("line{}", i), false);
        }
        q.flush();
        assert_eq!(q.index, 10);
        q.prev(1);
        assert_eq!(q.index, 9);
        assert_eq!(q.current(), Some("line9".to_string()));
        q.push("new_line".to_string(), false);
        assert_eq!(q.index, 8);
        assert_eq!(q.current(), Some("line9".to_string()));
        for _ in 0..9 {
            q.push("new_line".to_string(), false);
        }
        assert_eq!(q.index, 0);
        assert_eq!(q.current(), Some("new_line".to_string()));
    }

    #[test]
    fn test_scan_index() {
        let mut q = SpeechQueue::new(10);
        for i in 0..10 {
            q.push(format!("line{}", i), false);
        }
        assert_eq!(q.scan_index, 10);

        assert_eq!(q.scan_back(5), Some("line5".to_string()));
        assert_eq!(q.scan_index, 5);
        q.push("new_line".to_string(), false);
        assert_eq!(q.scan_index, 4);
        assert_eq!(q.scan_forward(3), Some("line8".to_string()));
        assert_eq!(q.scan_index, 7);
        q.scan_forward(100);
        assert_eq!(q.scan_index, 10);
    }

    #[test]
    fn test_push_no_blank() {
        let mut q = SpeechQueue::new(100);
        assert_eq!(q.push("".to_string(), false), None);
        assert_eq!(q.push("test".to_string(), false), Some("test".to_string()));
        assert_eq!(q.push("".to_string(), true), None);
    }

    #[test]
    fn test_next_no_blank() {
        let mut q = SpeechQueue::new(100);
        q.push("line".to_string(), false);
        for _ in 0..10 {
            q.push("".to_string(), false);
        }
        q.push("line".to_string(), false);
        assert_eq!(q.next(1), Some("line".to_string()));
    }

    #[test]
    fn test_prev_no_blank() {
        let mut q = SpeechQueue::new(100);
        q.push("line".to_string(), false);
        for _ in 0..10 {
            q.push("".to_string(), false);
        }
        q.flush();
        assert_eq!(q.prev(1), Some("line".to_string()));
    }
}
