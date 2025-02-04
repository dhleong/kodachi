use crate::app::processing::ansi::Ansi;

use super::{
    duplex::{word_index::WordIndexSelectorFactory, DuplexCompletionSource},
    filtering::FilteringCompletionSource,
    recency::RecencyCompletionSource,
    sent::SentCompletionSource,
    transforms::WordTransform,
    CompletionParams, CompletionSource,
};

pub struct Completions {
    source: DuplexCompletionSource<
        SentCompletionSource,
        FilteringCompletionSource<RecencyCompletionSource>,
        WordIndexSelectorFactory,
    >,
}

impl Default for Completions {
    fn default() -> Self {
        Self {
            source: DuplexCompletionSource::new(
                SentCompletionSource::default(),
                FilteringCompletionSource(RecencyCompletionSource::default()),
                WordIndexSelectorFactory::with_weights_by_index(vec![
                    // First word? Prefer commandCompletions ALWAYS; We'll still
                    // fallback to output if commandCompletion doesn't have anything
                    (100, 0),
                    // Second word? Actually, prefer output a bit
                    // eg: get <thing>; enter <thing>; look <thing>
                    (35, 65),
                    // Otherwise, just split it evenly
                    (50, 50),
                ]),
            ),
        }
    }
}

impl Completions {
    pub fn process_incoming(&mut self, line: &mut Ansi) {
        self.source.second.0.process_line(&line.strip_ansi())
    }

    pub fn process_outgoing(&mut self, line: String) {
        self.source.first.process_outgoing(&line);
    }

    pub fn suggest(&self, params: CompletionParams) -> impl Iterator<Item = String> + '_ {
        let transformer = WordTransform::matching_word(params.word_to_complete());
        self.source
            .suggest(params)
            .map(move |s| transformer.transform(s))
    }
}
