#[allow(dead_code)]
mod completion_tree;

pub use completion_tree::CompletionTree;

#[cfg(test)]
mod tests {
    use crate::completion_tree::CompletionTree;

    #[test]
    fn test_completion() {
        let mut tree = CompletionTree::new();
        tree.insert("wording");
        let completions = tree.complete("wo").unwrap_or(vec![]);
        let mut iter = completions.iter();
        assert_eq!(iter.next(), Some(&"wording".to_string()));
    }

    #[test]
    fn test_multi_completion() {
        let mut tree = CompletionTree::new();
        tree.insert("wording");
        tree.insert("wollybugger");
        tree.insert("workerbee");
        tree.insert("worldleader");
        tree.insert("batman");
        tree.insert("robin");
        let completions = tree.complete("wo").unwrap();
        assert!(completions.contains(&"workerbee".to_string()));
        assert!(completions.contains(&"wollybugger".to_string()));
        assert!(completions.contains(&"wording".to_string()));
        assert!(completions.contains(&"worldleader".to_string()));
        assert!(!completions.contains(&"batman".to_string()));
        assert!(!completions.contains(&"robin".to_string()));
    }

    #[test]
    fn test_multi_insert() {
        let mut tree = CompletionTree::new();
        tree.insert("wollybugger workerbee worldleader batman robin wording");
        let completions = tree.complete("wo").unwrap();
        assert!(completions.contains(&"workerbee".to_string()));
        assert!(completions.contains(&"wollybugger".to_string()));
        assert!(completions.contains(&"wording".to_string()));
        assert!(completions.contains(&"worldleader".to_string()));
        assert!(!completions.contains(&"batman".to_string()));
        assert!(!completions.contains(&"robin".to_string()));
    }

    #[test]
    fn test_substring_matches() {
        let mut tree = CompletionTree::new();
        tree.insert("dumpster dumpsterfire");
        let completions = tree.complete("dum").unwrap();
        assert!(completions.contains(&"dumpster".to_string()));
        assert!(completions.contains(&"dumpsterfire".to_string()));
    }

    #[test]
    fn test_dont_include_specials() {
        let mut tree = CompletionTree::new();
        tree.insert("dumpster\x1b[34m dumpsterfire{}");
        let completions = tree.complete("dum").unwrap();
        assert!(completions.contains(&"dumpster".to_string()));
        assert!(completions.contains(&"dumpsterfire".to_string()));
    }

    #[test]
    fn test_without_inclusions() {
        let mut tree = CompletionTree::new();
        tree.insert("/dumpster /dumpsterfire");
        assert!(tree.complete("/dum").is_none());
    }

    #[test]
    fn test_with_inclusions() {
        let mut tree = CompletionTree::with_inclusions(&['/', '_']);
        tree.insert("/dumpster /dumpster_fire");
        let completions = tree.complete("/dum").unwrap();
        assert!(completions.contains(&"/dumpster".to_string()));
        assert!(completions.contains(&"/dumpster_fire".to_string()));
    }
}
