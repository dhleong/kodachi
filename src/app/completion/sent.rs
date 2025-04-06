use super::{
    duplex::{word_index::WordIndexSelectorFactory, DuplexCompletionSource},
    filtering::FilteringCompletionSource,
    markov::MarkovCompletionSource,
    recency::RecencyCompletionSource,
    CompletionParams, CompletionSource,
};

pub struct SentCompletionSource(
    DuplexCompletionSource<
        FilteringCompletionSource<MarkovCompletionSource>,
        FilteringCompletionSource<RecencyCompletionSource>,
        WordIndexSelectorFactory,
    >,
);

impl Default for SentCompletionSource {
    fn default() -> Self {
        Self(DuplexCompletionSource::new(
            FilteringCompletionSource(MarkovCompletionSource::default()),
            FilteringCompletionSource(RecencyCompletionSource::default()),
            WordIndexSelectorFactory::with_weights_by_index([
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
        FilteringCompletionSource<MarkovCompletionSource>,
        FilteringCompletionSource<RecencyCompletionSource>,
        WordIndexSelectorFactory,
    > as CompletionSource>::Iter<'a>;

    fn suggest(&self, params: CompletionParams) -> Self::Iter<'_> {
        self.0.suggest(params)
    }
}

impl SentCompletionSource {
    pub fn process_outgoing(&mut self, line: &str) {
        self.0.first.0.process_line(line);
        self.0.second.0.process_line(line);
    }
}
