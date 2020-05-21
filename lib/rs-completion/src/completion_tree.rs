use std::collections::{BTreeMap, BTreeSet};
use std::{rc::Rc, str::Chars};

#[derive(Debug, Clone)]
pub struct CompletionTree {
    root: CompletionNode,
    inclusions: Rc<BTreeSet<char>>,
}

impl Default for CompletionTree {
    fn default() -> Self {
        let inclusions = Rc::new(BTreeSet::new());
        Self {
            root: CompletionNode::new(inclusions.clone()),
            inclusions,
        }
    }
}

impl CompletionTree {
    pub fn with_inclusions(incl: &[char]) -> Self {
        let mut set = BTreeSet::new();
        incl.iter().for_each(|c| {
            set.insert(*c);
        });
        let inclusions = Rc::new(set);
        Self {
            root: CompletionNode::new(inclusions.clone()),
            inclusions,
        }
    }

    pub fn insert(&mut self, line: &str) {
        for word in line.split_whitespace() {
            if word.len() > 4 {
                self.root.insert(word.chars());
            }
        }
    }

    pub fn complete(&self, line: &str) -> Option<Vec<String>> {
        if !line.is_empty() {
            let last_word = line.split_whitespace().last().unwrap();
            if let Some(extensions) = self.root.complete(last_word.chars()) {
                return Some(
                    extensions
                        .iter()
                        .map(|ext| format!("{}{}", line, ext))
                        .collect::<Vec<String>>(),
                );
            } else {
                return None;
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
struct CompletionNode {
    subnodes: BTreeMap<char, CompletionNode>,
    leaf: bool,
    inclusions: Rc<BTreeSet<char>>,
}

impl CompletionNode {
    fn new(incl: Rc<BTreeSet<char>>) -> Self {
        Self {
            subnodes: BTreeMap::new(),
            leaf: false,
            inclusions: incl,
        }
    }

    fn insert(&mut self, mut iter: Chars) {
        if let Some(c) = iter.next() {
            if self.inclusions.contains(&c) || c.is_alphanumeric() {
                let inclusions = self.inclusions.clone();
                let subnode = self
                    .subnodes
                    .entry(c)
                    .or_insert_with(|| CompletionNode::new(inclusions));
                subnode.insert(iter);
            } else {
                self.leaf = true;
            }
        } else {
            self.leaf = true;
        }
    }

    fn complete(&self, mut iter: Chars) -> Option<Vec<String>> {
        if let Some(c) = iter.next() {
            if let Some(subnode) = self.subnodes.get(&c) {
                subnode.complete(iter)
            } else {
                None
            }
        } else {
            Some(self.collect("".to_string()))
        }
    }

    fn collect(&self, partial: String) -> Vec<String> {
        let mut completions = vec![];
        if self.leaf {
            completions.push(partial.clone());
        }

        if !self.subnodes.is_empty() {
            for (c, node) in &self.subnodes {
                let mut partial = partial.clone();
                partial.push(*c);
                completions.append(&mut node.collect(partial));
            }
        }
        completions
    }
}
