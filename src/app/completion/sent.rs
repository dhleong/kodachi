use super::{
    duplex::{word_index::WordIndexSelectorFactory, DuplexCompletionSource},
    markov::MarkovCompletionSource,
    recency::RecencyCompletionSource,
    CompletionParams, CompletionSource,
};

pub struct SentCompletionSource(
    DuplexCompletionSource<
        MarkovCompletionSource,
        RecencyCompletionSource,
        WordIndexSelectorFactory,
    >,
);

impl Default for SentCompletionSource {
    fn default() -> Self {
        Self(DuplexCompletionSource::new(
            MarkovCompletionSource::default(),
            RecencyCompletionSource::default(),
            WordIndexSelectorFactory::with_weights_by_index(vec![
                // The markov trie has a max depth of 5; at that point, we start to suspect
                // that it's not a structured command, so we let recency have more weight
                (100, 0),
                (100, 0),
                (100, 0),
                (100, 0),
                // After the first few words, still prefer markov, but
                // give recent words a bit of a chance, too
                (50, 50),
            ]),
        ))
    }
}

impl CompletionSource for SentCompletionSource {
    type Iter<'a> = <DuplexCompletionSource<
        MarkovCompletionSource,
        RecencyCompletionSource,
        WordIndexSelectorFactory,
    > as CompletionSource>::Iter<'a>;

    fn suggest<'a>(&'a self, params: CompletionParams) -> Self::Iter<'a> {
        self.0.suggest(params)
    }
}

impl SentCompletionSource {
    pub fn process_outgoing(&mut self, line: &str) {
        self.0.first.process_line(line);
        self.0.second.process_line(line);
    }
}
