use crate::model::{Line, Regex, TagMask};

pub struct History {
    inner: Vec<Line>,
    visible: Vec<Line>,
    tag_mask: TagMask,
    pub capacity: usize,
    pub drain_length: usize,
}

impl History {
    pub fn new() -> Self {
        let drain_length: usize = 1024;
        let capacity: usize = 32 * drain_length;
        Self {
            inner: Vec::with_capacity(capacity),
            visible: Vec::with_capacity(capacity),
            tag_mask: TagMask::default(),
            capacity,
            drain_length,
        }
    }

    fn rebuild_visible(&mut self) {
        self.visible = self
            .inner
            .iter()
            .filter(|l| !l.is_masked(&self.tag_mask))
            .cloned()
            .collect();
    }

    pub fn set_tag_mask(&mut self, mask: TagMask) {
        self.tag_mask = mask;
        self.rebuild_visible();
    }

    pub fn set_capacity(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        if self.inner.len() > self.capacity {
            let excess = self.inner.len() - self.capacity;
            let drained: Vec<Line> = self.inner.drain(0..excess).collect();
            let visible_drain_count = drained
                .iter()
                .filter(|l| !l.is_masked(&self.tag_mask))
                .count();
            if visible_drain_count > 0 {
                if visible_drain_count <= self.visible.len() {
                    self.visible.drain(0..visible_drain_count);
                } else {
                    self.rebuild_visible();
                }
            }
        }
    }

    pub fn drain(&mut self) {
        if self.inner.len() >= self.capacity {
            let drain_len = self.drain_length.min(self.inner.len());
            let drained: Vec<Line> = self.inner.drain(0..drain_len).collect();
            let visible_drain_count = drained
                .iter()
                .filter(|l| !l.is_masked(&self.tag_mask))
                .count();
            if visible_drain_count > 0 {
                if visible_drain_count <= self.visible.len() {
                    self.visible.drain(0..visible_drain_count);
                } else {
                    self.rebuild_visible();
                }
            }
        }
    }

    pub fn append_str(&mut self, line: &str) {
        self.append(line);
    }

    pub fn append(&mut self, line: &str) {
        if !line.trim().is_empty() {
            for segment in line.lines() {
                let l = Line::from(segment);
                if !l.is_masked(&self.tag_mask) {
                    self.visible.push(l.clone());
                }
                self.inner.push(l);
            }
        } else {
            let l = Line::from("");
            if !l.is_masked(&self.tag_mask) {
                self.visible.push(l.clone());
            }
            self.inner.push(l);
        }
        self.drain();
    }

    pub fn append_line(&mut self, line: Line) {
        if !line.is_masked(&self.tag_mask) {
            self.visible.push(line.clone());
        }
        self.inner.push(line);
        self.drain();
    }

    pub fn remove_last_if_prefix(&mut self, line: &str) -> Option<Line> {
        if let Some(prefix) = self.inner.last() {
            if line.starts_with(prefix.line()) {
                let popped = self.inner.pop();
                if let Some(ref l) = popped {
                    if !l.is_masked(&self.tag_mask) {
                        self.visible.pop();
                    }
                }
                popped
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get(&self, index: usize) -> &Line {
        &self.visible[index]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Line> {
        self.visible.iter()
    }

    pub fn len(&self) -> usize {
        self.visible.len()
    }

    pub fn is_empty(&self) -> bool {
        self.visible.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.visible.clear();
    }

    pub fn find_forward(&self, pattern: &Regex, pos: usize) -> Option<usize> {
        self.visible[pos..]
            .iter()
            .position(|l| pattern.is_match(l.clean_line()))
            .map(|index| pos + index)
    }

    pub fn find_backward(&self, pattern: &Regex, pos: usize) -> Option<usize> {
        self.visible[..pos]
            .iter()
            .rev()
            .position(|l| pattern.is_match(l.clean_line()))
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

    #[test]
    fn test_append_line() {
        let mut history = History::new();
        let mut line = Line::from("hello world");
        line.tag.color = "\x1b[31m".to_string();
        history.append_line(line);
        assert_eq!(history.len(), 1);
        assert_eq!(history.inner[0].clean_line(), "hello world");
        assert_eq!(history.inner[0].tag.color, "\x1b[31m");
    }

    #[test]
    fn test_tag_mask_filters_visible() {
        let mut history = History::new();
        let mut masked = Line::from("combat line");
        masked.tag.key = "combat".to_string();
        let unmasked = Line::from("normal line");

        history.append_line(masked);
        history.append_line(unmasked);
        assert_eq!(history.len(), 2); // No mask yet

        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);

        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).clean_line(), "normal line");
    }

    #[test]
    fn test_tag_mask_inner_preserved() {
        let mut history = History::new();
        let mut masked = Line::from("combat line");
        masked.tag.key = "combat".to_string();
        history.append_line(masked);

        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);

        assert_eq!(history.len(), 0); // not visible
        assert_eq!(history.inner.len(), 1); // still in inner
    }

    #[test]
    fn test_tag_mask_reset_restores_visible() {
        let mut history = History::new();
        let mut masked = Line::from("combat line");
        masked.tag.key = "combat".to_string();
        history.append_line(masked);
        history.append_line(Line::from("normal line"));

        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);
        assert_eq!(history.len(), 1);

        history.set_tag_mask(TagMask::default());
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_append_line_with_active_mask() {
        let mut history = History::new();
        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);

        let mut masked = Line::from("combat line");
        masked.tag.key = "combat".to_string();
        history.append_line(masked);
        history.append_line(Line::from("normal line"));

        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).clean_line(), "normal line");
        assert_eq!(history.inner.len(), 2);
    }

    #[test]
    fn test_tag_mask_reverse_shows_only_matching() {
        let mut history = History::new();
        let mut combat = Line::from("combat line");
        combat.tag.key = "combat".to_string();
        history.append_line(combat);
        history.append_line(Line::from("normal line"));

        let mask = TagMask {
            key: Some("combat".to_string()),
            reverse: true,
            ..Default::default()
        };
        history.set_tag_mask(mask);

        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).clean_line(), "combat line");
    }

    #[test]
    fn test_remove_last_if_prefix_masked() {
        let mut history = History::new();
        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);

        history.append_line(Line::from("normal line"));
        let mut masked = Line::from("combat prompt");
        masked.tag.key = "combat".to_string();
        history.append_line(masked);

        assert_eq!(history.len(), 1); // only normal in visible
        history.remove_last_if_prefix("combat prompt extended");
        // masked line removed from inner, visible unchanged
        assert_eq!(history.len(), 1);
        assert_eq!(history.inner.len(), 1);
    }

    #[test]
    fn test_drain_incremental_no_mask() {
        let mut history = History::new();
        for i in 0..history.capacity - 1 {
            history.append(&format!("line {}", i));
        }
        assert_eq!(history.inner.len(), history.capacity - 1);
        let visible_before = history.len();
        history.append("overflow line");
        assert_eq!(history.inner.len(), history.capacity - history.drain_length);
        assert_eq!(history.len(), visible_before - history.drain_length + 1);
    }

    #[test]
    fn test_drain_incremental_with_mask() {
        let mut history = History::new();
        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);

        for i in 0..history.capacity - 1 {
            let mut line = Line::from(&format!("line {}", i));
            if i % 2 == 0 {
                line.tag.key = "combat".to_string();
            }
            history.append_line(line);
        }
        let visible_before = history.len();
        let inner_before = history.inner.len();
        assert_eq!(inner_before, history.capacity - 1);

        let mut overflow = Line::from("overflow");
        overflow.tag.key = "combat".to_string();
        history.append_line(overflow);

        assert_eq!(history.inner.len(), inner_before - history.drain_length + 1);
        let drained_visible = visible_before - history.len() + 1;
        assert!(drained_visible <= history.drain_length);
    }

    #[test]
    fn test_drain_visible_inner_consistency() {
        let mut history = History::new();
        for i in 0..history.capacity + 100 {
            history.append(&format!("line {}", i));
        }
        let visible_count = history.visible.len();
        let inner_count = history.inner.len();
        assert_eq!(visible_count, inner_count);
        for i in 0..visible_count {
            assert_eq!(history.visible[i].line(), history.inner[i].line());
        }
    }

    #[test]
    fn test_set_capacity_reduces_lines() {
        let mut history = History::new();
        for i in 0..1000 {
            history.append(&format!("line {}", i));
        }
        assert_eq!(history.inner.len(), 1000);
        assert_eq!(history.visible.len(), 1000);

        history.set_capacity(500);
        assert_eq!(history.capacity, 500);
        assert_eq!(history.inner.len(), 500);
        assert_eq!(history.visible.len(), 500);
        assert_eq!(history.visible[0].line(), "line 500");
    }

    #[test]
    fn test_set_capacity_with_mask() {
        let mut history = History::new();
        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);

        for i in 0..1000 {
            let mut line = Line::from(&format!("line {}", i));
            if i % 2 == 0 {
                line.tag.key = "combat".to_string();
            }
            history.append_line(line);
        }
        let visible_before = history.len();
        let inner_before = history.inner.len();
        assert_eq!(inner_before, 1000);
        assert!(visible_before < inner_before);

        history.set_capacity(500);
        assert_eq!(history.capacity, 500);
        assert_eq!(history.inner.len(), 500);
        let visible_after = history.len();
        assert!(visible_after < visible_before);
    }

    #[test]
    fn test_set_capacity_larger_than_current() {
        let mut history = History::new();
        for i in 0..100 {
            history.append(&format!("line {}", i));
        }
        assert_eq!(history.inner.len(), 100);

        history.set_capacity(500);
        assert_eq!(history.capacity, 500);
        assert_eq!(history.inner.len(), 100);
        assert_eq!(history.visible.len(), 100);
    }

    #[test]
    fn test_drain_with_heavy_mask_no_panic() {
        let mut history = History::new();
        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);

        for i in 0..history.capacity + 100 {
            let mut line = Line::from(&format!("line {}", i));
            if i % 10 != 0 {
                line.tag.key = "combat".to_string();
            }
            history.append_line(line);
        }

        assert!(history.inner.len() <= history.capacity);
        assert!(history.visible.len() <= history.inner.len());
        for i in 0..history.visible.len() {
            assert!(!history.visible[i].is_masked(&history.tag_mask));
        }
    }

    #[test]
    fn test_set_capacity_with_heavy_mask_no_panic() {
        let mut history = History::new();
        let mask = TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        };
        history.set_tag_mask(mask);

        for i in 0..1000 {
            let mut line = Line::from(&format!("line {}", i));
            if i % 10 != 0 {
                line.tag.key = "combat".to_string();
            }
            history.append_line(line);
        }

        history.set_capacity(100);
        assert_eq!(history.inner.len(), 100);
        assert!(history.visible.len() <= history.inner.len());
        for i in 0..history.visible.len() {
            assert!(!history.visible[i].is_masked(&history.tag_mask));
        }
    }
}
