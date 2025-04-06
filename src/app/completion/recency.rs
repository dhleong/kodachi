use crate::app::history::History;

use super::{tokens::Tokens, CompletionParams, CompletionSource};

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
        self.history.insert_many(&Tokens::significant_from(line));
    }
}

impl CompletionSource for RecencyCompletionSource {
    type Iter<'a> = ritelinked::linked_hash_set::Iter<'a, String>;

    fn suggest(&self, _params: CompletionParams) -> Self::Iter<'_> {
        self.history.iter()
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
        assert_eq!(suggestions, vec!["the", "honor"]);
    }

    #[test]
    fn case_insensitivity() {
        let mut completions = RecencyCompletionSource::with_capacity(2);
        completions.process_line("For The HONOR");
        let suggestions: Vec<&String> = completions.suggest(CompletionParams::empty()).collect();
        assert_eq!(suggestions, vec!["the", "honor"]);
    }
}
