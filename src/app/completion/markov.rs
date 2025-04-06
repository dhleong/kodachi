use crate::collections::markov_trie::{MarkovTrie, QueryNext};

use super::{tokens::Tokens, CompletionParams, CompletionSource};

#[derive(Default)]
pub struct MarkovCompletionSource {
    trie: MarkovTrie<String>,
}

impl MarkovCompletionSource {
    pub fn process_line<S: AsRef<str>>(&mut self, line: S) {
        self.trie.add_sequence(&Tokens::from(line.as_ref()));
    }
}

impl CompletionSource for MarkovCompletionSource {
    type Iter<'a> = <QueryNext<'a, String> as IntoIterator>::IntoIter;

    fn suggest(&self, params: CompletionParams) -> Self::Iter<'_> {
        let words: Vec<String> = params
            .words_before_cursor()
            .iter()
            .map(|s| s.to_string())
            .collect();
        self.trie.query_next(&words).into_iter()
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

    fn suggest_vec(source: &MarkovCompletionSource, query: CompletionParams) -> Vec<&String> {
        source.suggest(query).collect()
    }

    #[test]
    fn first_completion_test() {
        let source = source();
        let params = CompletionParams::from_word("t");
        assert_eq!(
            source
                .suggest(params)
                .next()
                .expect("Expected to have a suggestion")
                .to_string(),
            "take"
        );
    }

    #[test]
    fn sequence_completion_test() {
        let source = source();
        let params = CompletionParams::from_line_to_cursor("take m");
        assert_eq!(suggest_vec(&source, params.clone()), vec!["my", "me"]);

        // Do it one more time to prove it wasn't a fluke
        assert_eq!(suggest_vec(&source, params), vec!["my", "me"]);
    }
}
