use regex::Regex;

use crate::app::history::History;

use super::{CompletionParams, CompletionSource};

const DEFAULT_RECENCY_CAPACITY: usize = 5000;

pub struct RecencyCompletionSource {
    history: History<String>,
}

impl Default for RecencyCompletionSource {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_RECENCY_CAPACITY)
    }
}

impl RecencyCompletionSource {
    pub fn with_capacity(capacity: usize) -> Self {
        RecencyCompletionSource {
            history: History::with_capacity(capacity),
        }
    }

    pub fn process_line(&mut self, line: &str) {
        let words_regex = Regex::new(r"(\w+)").unwrap();
        self.history.insert_many(
            words_regex
                .find_iter(line)
                .map(|m| m.as_str().to_lowercase()),
        );
    }
}

impl CompletionSource for RecencyCompletionSource {
    type Iter<'a> = std::iter::Rev<ritelinked::linked_hash_set::Iter<'a, String>>;

    fn suggest<'a>(&'a self, _params: CompletionParams) -> Self::Iter<'a> {
        // NOTE: We need to suggest in reverse order so the most recently received content is
        // suggested *first*
        self.history.iter().rev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_test() {
        let mut completions = RecencyCompletionSource::with_capacity(2);
        completions.process_line("for the honor");
        let suggestions: Vec<&String> = completions.suggest(CompletionParams::empty()).collect();
        assert_eq!(suggestions, vec!["honor", "the"]);
    }

    #[test]
    fn case_insensitivity() {
        let mut completions = RecencyCompletionSource::with_capacity(2);
        completions.process_line("For The HONOR");
        let suggestions: Vec<&String> = completions.suggest(CompletionParams::empty()).collect();
        assert_eq!(suggestions, vec!["honor", "the"]);
    }

    #[test]
    fn recency_test() {
        let mut completions = RecencyCompletionSource::with_capacity(10);
        completions.process_line("take my love take my land");
        let suggestions: Vec<&String> = completions.suggest(CompletionParams::empty()).collect();
        assert_eq!(suggestions, vec!["land", "my", "take", "love"]);
    }
}
