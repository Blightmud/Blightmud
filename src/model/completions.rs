#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Completions {
    inner: Vec<String>,
    lock: bool,
}

impl Completions {
    pub fn lock(&mut self, lock: bool) {
        self.lock = lock;
    }

    #[cfg(test)]
    pub fn get(&self, idx: usize) -> Option<&String> {
        self.inner.get(idx)
    }

    pub fn merge(&mut self, mut other: Self) {
        if !self.lock {
            self.inner.append(&mut other.inner)
        }
        self.lock = other.lock;
    }

    pub fn add_all(&mut self, other: &mut Vec<String>) {
        if !self.lock {
            self.inner.append(other)
        }
    }

    #[cfg(test)]
    pub fn push(&mut self, item: String) {
        if !self.lock {
            self.inner.push(item);
        }
    }

    pub fn iter(&'_ self) -> std::slice::Iter<'_, String> {
        self.inner.iter()
    }
}

impl From<Vec<String>> for Completions {
    fn from(v: Vec<String>) -> Self {
        Self {
            inner: v,
            lock: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Completions;

    #[test]
    fn confirm_lock() {
        let mut completions = Completions::default();
        completions.push("test".to_string());
        assert_eq!(completions.get(0), Some(&"test".to_string()));
        assert_eq!(completions.get(1), None);
        completions.lock(true);
        completions.add_all(&mut vec!["test".to_string()]);
        completions.merge(Completions::from(vec!["test".to_string()]));

        let mut it = completions.iter();
        assert_eq!(it.next(), Some(&"test".to_string()));
        assert_eq!(it.next(), None);

        completions.lock(false);
        completions.add_all(&mut vec!["test".to_string()]);
        completions.merge(Completions::from(vec!["test".to_string()]));

        let mut it = completions.iter();
        assert_eq!(it.next(), Some(&"test".to_string()));
        assert_eq!(it.next(), Some(&"test".to_string()));
        assert_eq!(it.next(), Some(&"test".to_string()));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn confirm_merge() {
        let mut completions = Completions::default();
        assert_eq!(completions.lock, false);

        let mut other = Completions::default();
        other.lock(true);
        completions.merge(other);
        assert_eq!(completions.lock, true);
    }
}
