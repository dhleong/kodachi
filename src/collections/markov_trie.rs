// TODO: Remove this:
#![allow(unused)]

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
    fn query_next(&self, sequence: &[T]) -> Vec<&T> {
        if let Some(leaf) = self.root.find_node(sequence) {
            let mut candidate_nodes: Vec<&MarkovNode<T>> =
                leaf.transitions.transitions.values().collect();
            candidate_nodes.sort_by_key(|node| node.incoming_count);
            candidate_nodes.iter().map(|node| &node.value).collect()
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

    fn find_node(&self, sequence: &[T]) -> Option<&MarkovNode<T>> {
        if sequence.is_empty() {
            None
        } else {
            let next_value = &sequence[0];
            if let Some(next_node) = self.transitions.get(next_value) {
                if sequence.len() < 1 {
                    Some(next_node)
                } else {
                    next_node.transitions.find_node(&sequence[1..])
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
