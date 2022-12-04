use regex::Regex;

use ritelinked::LinkedHashSet;

use crate::app::processing::ansi::Ansi;

use super::{transforms::WordTransform, CompletionParams};

const DEFAULT_RECENCY_CAPACITY: usize = 5000;

pub struct Completions {
    max_entries: usize,
    incoming_words: LinkedHashSet<String>,
}

impl Default for Completions {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_RECENCY_CAPACITY)
    }
}

impl Completions {
    pub fn with_capacity(capacity: usize) -> Self {
        Completions {
            max_entries: capacity,
            incoming_words: LinkedHashSet::with_capacity(capacity),
        }
    }

    pub fn process_incoming(&mut self, line: &mut Ansi) {
        let words_regex = Regex::new(r"(\w+)").unwrap();
        for m in words_regex.find_iter(&line.strip_ansi()) {
            let word = m.as_str();
            if !self.incoming_words.contains(word) {
                self.incoming_words.insert(word.to_string());
            }
        }

        if let Some(overage) = self.incoming_words.len().checked_sub(self.max_entries) {
            for _ in 0..overage {
                self.incoming_words.pop_front();
            }
        }
    }

    pub fn suggest(&self, params: CompletionParams) -> Vec<String> {
        let transformer = WordTransform::matching_word(params.word_to_complete);
        self.incoming_words
            .iter()
            .map(|s| transformer.transform(s))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_test() {
        let params = CompletionParams {
            word_to_complete: "".to_string(),
            line_to_cursor: "".to_string(),
            line: "".to_string(),
        };

        let mut completions = Completions::with_capacity(2);
        completions.process_incoming(&mut Ansi::from("for the honor"));
        let suggestions = completions.suggest(params);
        assert_eq!(suggestions, vec!["the", "honor"]);
    }
}
