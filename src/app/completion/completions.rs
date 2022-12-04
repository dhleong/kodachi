use regex::Regex;

use crate::app::{history::History, processing::ansi::Ansi};

use super::{transforms::WordTransform, CompletionParams};

const DEFAULT_RECENCY_CAPACITY: usize = 5000;

pub struct Completions {
    history: History<String>,
}

impl Default for Completions {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_RECENCY_CAPACITY)
    }
}

impl Completions {
    pub fn with_capacity(capacity: usize) -> Self {
        Completions {
            history: History::with_capacity(capacity),
        }
    }

    pub fn process_incoming(&mut self, line: &mut Ansi) {
        let words_regex = Regex::new(r"(\w+)").unwrap();
        self.history.insert_many(
            words_regex
                .find_iter(&line.strip_ansi())
                .map(|m| m.as_str().to_string()),
        );
    }

    pub fn process_outgoing(&mut self, line: String) {
        let words_regex = Regex::new(r"(\w+)").unwrap();
        self.history
            .insert_many(words_regex.find_iter(&line).map(|m| m.as_str().to_string()));
    }

    pub fn suggest(&self, params: CompletionParams) -> Vec<String> {
        let transformer = WordTransform::matching_word(params.word_to_complete);
        self.history
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
