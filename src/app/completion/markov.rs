use crate::collections::markov_trie::MarkovTrie;

use super::CompletionParams;

#[derive(Default)]
pub struct MarkovCompletionSource {
    trie: MarkovTrie<String>,
}

impl MarkovCompletionSource {
    pub fn process_line<S: AsRef<str>>(&mut self, line: S) {
        let words: Vec<String> = line
            .as_ref()
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        self.trie.add_sequence(&words);
    }

    pub fn suggest(&self, params: CompletionParams) -> Vec<&String> {
        let words: Vec<String> = params
            .words_before_cursor()
            .iter()
            .map(|s| s.to_string())
            .collect();
        self.trie.query_next(&words)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> MarkovCompletionSource {
        let mut source = MarkovCompletionSource::default();

        // Process in weird order to demonstrate it's frequency based,
        // not insert-order or alpha or anything
        source.process_line("Take my love");
        source.process_line("I'm still free");
        source.process_line("Take me where I cannot stand");
        source.process_line("I don't care");
        source.process_line("Take my land");

        return source;
    }

    #[test]
    fn first_completion_test() {
        let source = source();
        let params = CompletionParams {
            word_to_complete: "t".to_string(),
            line_to_cursor: "t".to_string(),
            line: "t".to_string(),
        };
        assert_eq!(
            source
                .suggest(params)
                .first()
                .expect("Expected to have a suggestion")
                .to_string(),
            "take"
        );
    }

    #[test]
    fn sequence_completion_test() {
        let source = source();
        let params = CompletionParams {
            word_to_complete: "m".to_string(),
            line_to_cursor: "take m".to_string(),
            line: "take m".to_string(),
        };
        assert_eq!(source.suggest(params), vec!["my", "me"]);
    }
}
