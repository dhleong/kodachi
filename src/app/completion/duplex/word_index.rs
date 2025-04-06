use crate::app::completion::CompletionParams;

use super::{
    weighted::{RandomnessSource, ThreadRngRandomnessSource, WeightedRandomSelectorFactory},
    DuplexSelectorFactory,
};

pub struct WordIndexSelectorFactory<
    R: RandomnessSource + Send + 'static = ThreadRngRandomnessSource,
> {
    weights_by_index: Vec<(u8, u8)>,
    random: R,
}

impl WordIndexSelectorFactory {
    pub fn with_weights_by_index(weights_by_index: impl Into<Vec<(u8, u8)>>) -> Self {
        Self {
            weights_by_index: weights_by_index.into(),
            random: ThreadRngRandomnessSource,
        }
    }

    #[allow(dead_code)]
    pub fn with_random<R: RandomnessSource + Send + 'static>(
        self,
        random: R,
    ) -> WordIndexSelectorFactory<R> {
        WordIndexSelectorFactory {
            weights_by_index: self.weights_by_index,
            random,
        }
    }
}

impl<R: RandomnessSource + Send + 'static> DuplexSelectorFactory for WordIndexSelectorFactory<R> {
    type Selector = <WeightedRandomSelectorFactory<R> as DuplexSelectorFactory>::Selector;

    fn create(&self, params: CompletionParams) -> Self::Selector {
        let index = params.word_index().min(self.weights_by_index.len() - 1);
        let (first, second) = &self.weights_by_index[index];
        WeightedRandomSelectorFactory::with_weights(*first, *second)
            .with_random(self.random.clone())
            .create(params)
    }
}

#[cfg(test)]
mod tests {
    use crate::app::completion::duplex::{
        weighted::StaticRandomnessSource, DuplexSelector, SelectionResult,
    };

    use super::*;

    #[test]
    fn empty_params_test() {
        let factory = WordIndexSelectorFactory::with_weights_by_index(vec![(100, 0), (0, 100)]);
        let mut empty = factory.create(CompletionParams::empty());

        // For the first word, we should always select the first result
        assert_eq!(empty.select(), SelectionResult::First);
    }

    #[test]
    fn first_word_test() {
        let factory = WordIndexSelectorFactory::with_weights_by_index(vec![(100, 0), (0, 100)]);
        let mut empty = factory.create(CompletionParams::from_line_to_cursor("fir"));

        // For the first word, we should always select the first result
        assert_eq!(empty.select(), SelectionResult::First);
    }

    #[test]
    fn second_word_test() {
        // FIXME: *Probably* if something has a 0% weight,
        // we should *never* accept it, even if we roll a 0.
        let factory = WordIndexSelectorFactory::with_weights_by_index(vec![(100, 0), (0, 100)])
            .with_random(StaticRandomnessSource::with_values(vec![1]));
        let mut empty = factory.create(CompletionParams::from_line_to_cursor("first "));
        assert_eq!(empty.select(), SelectionResult::Second);
    }
}
