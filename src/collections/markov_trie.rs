// TODO: Remove this:
#![allow(unused)]

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

const DEFAULT_MAX_DEPTH: usize = 5;

pub struct MarkovTrie<T> {
    root: MarkovTransitions<T>,
    max_depth: usize,
    stop_words: Option<HashSet<T>>,
}

impl<T: Default> MarkovTrie<T> {
    fn with_stop_words(stop_words: HashSet<T>) -> Self {
        Self {
            root: Default::default(),
            max_depth: DEFAULT_MAX_DEPTH,
            stop_words: Some(stop_words),
        }
    }
}

impl<T: Default> Default for MarkovTrie<T> {
    fn default() -> Self {
        MarkovTrie::with_stop_words(Default::default())
    }
}

impl<T: Default + Hash + Eq + Clone> MarkovTrie<T> {
    // TODO: Can we use T: Borrow<Q> to support querying via &[&str] ?
    fn query_next(&self, sequence: &[T]) -> Vec<&T> {
        if sequence.is_empty() {
            // Special case: querying root node
            self.root.gather_transitions()
        } else if let Some(leaf) = self.root.find_node(sequence) {
            leaf.transitions.gather_transitions()
        } else {
            vec![]
        }
    }

    fn add_sequence(&mut self, sequence: &[T]) {
        if sequence.is_empty() {
            return;
        }

        self.root
            .add_sequence(sequence, self.stop_words.as_ref(), self.max_depth);
    }
}

#[derive(Default)]
struct MarkovTransitions<T> {
    transitions: HashMap<T, MarkovNode<T>>,
}

impl<T: Default + Hash + Eq + Clone> MarkovTransitions<T> {
    fn add_sequence(
        &mut self,
        mut sequence: &[T],
        stop_words: Option<&HashSet<T>>,
        remaining_depth: usize,
    ) {
        let next_value = &sequence[0];
        if stop_words.map_or(false, |stop_words| stop_words.contains(&next_value)) {
            return;
        }

        let mut transition = if let Some(existing) = self.transitions.get_mut(&next_value) {
            existing
        } else {
            self.transitions
                .entry(next_value.clone())
                .or_insert_with_key(|key| MarkovNode::from(key.clone()))
        };
        transition.incoming_count += 1;

        if let Some(new_remaining_depth) = remaining_depth.checked_sub(1) {
            if sequence.len() > 1 {
                transition.transitions.add_sequence(
                    &sequence[1..],
                    stop_words,
                    new_remaining_depth,
                );
            }
        }
    }

    fn gather_transitions(&self) -> Vec<&T> {
        let mut candidate_nodes: Vec<&MarkovNode<T>> = self.transitions.values().collect();
        candidate_nodes.sort_by_key(|node| Reverse(node.incoming_count));
        candidate_nodes.iter().map(|node| &node.value).collect()
    }

    fn find_node(&self, sequence: &[T]) -> Option<&MarkovNode<T>> {
        if sequence.is_empty() {
            None
        } else {
            let next_value = &sequence[0];
            if let Some(next_node) = self.transitions.get(next_value) {
                let remaining_sequence = &sequence[1..];
                if remaining_sequence.is_empty() {
                    Some(next_node)
                } else {
                    next_node.transitions.find_node(remaining_sequence)
                }
            } else {
                None
            }
        }
    }
}

struct MarkovNode<T> {
    pub value: T,
    pub incoming_count: usize,
    pub transitions: MarkovTransitions<T>,
}

impl<T: Default> From<T> for MarkovNode<T> {
    fn from(value: T) -> Self {
        MarkovNode {
            value,
            incoming_count: 0,
            transitions: MarkovTransitions::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(trie: &mut MarkovTrie<String>, phrase: &str) {
        let vec: Vec<String> = phrase.split_whitespace().map(|s| s.to_string()).collect();
        trie.add_sequence(&vec);
    }

    fn trie() -> MarkovTrie<String> {
        let mut trie = MarkovTrie::default();

        // Process in weird order to demonstrate it's frequency based,
        // not insert-order or alpha or anything
        process(&mut trie, "Take my love");
        process(&mut trie, "I'm still free");
        process(&mut trie, "Take me where I cannot stand");
        process(&mut trie, "I don't care");
        process(&mut trie, "Take my land");

        return trie;
    }

    #[test]
    pub fn first_completions() {
        let mut source = trie();
        let suggestions = source.query_next(&[]);
        assert_eq!(suggestions[0], "Take");
    }

    #[test]
    pub fn sequence_completion() {
        let mut source = trie();
        let suggestions = source.query_next(&["Take".to_string()]);
        assert_eq!(suggestions, vec!["my", "me"]);
    }

    #[test]
    pub fn ignore_stop_words() {
        let mut stop_words: HashSet<String> = HashSet::default();
        stop_words.insert("say".into());

        let mut source = MarkovTrie::with_stop_words(stop_words);
        process(&mut source, "say Hello");

        let suggestions = source.query_next(&[]);
        assert!(suggestions.is_empty());
    }
}
