use crate::app::completion::CompletionParams;

use super::{weighted::WeightedRandomSelectorFactory, DuplexSelectorFactory};

pub struct WordIndexSelectorFactory {
    weights_by_index: Vec<(u8, u8)>,
}

impl WordIndexSelectorFactory {
    pub fn with_weights_by_index(weights_by_index: Vec<(u8, u8)>) -> Self {
        Self { weights_by_index }
    }
}

impl DuplexSelectorFactory for WordIndexSelectorFactory {
    type Selector = <WeightedRandomSelectorFactory as DuplexSelectorFactory>::Selector;

    fn create(&self, params: CompletionParams) -> Self::Selector {
        let index = params.word_index().min(self.weights_by_index.len() - 1);
        let (first, second) = &self.weights_by_index[index];
        WeightedRandomSelectorFactory::with_weights(*first, *second).create(params)
    }
}

#[cfg(test)]
mod tests {
    use crate::app::completion::duplex::{DuplexSelector, SelectionResult};

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
        let mut empty = factory.create(CompletionParams {
            word_to_complete: "fir".to_string(),
            line_to_cursor: "fir".to_string(),
            line: "fir ".to_string(),
        });

        // For the first word, we should always select the first result
        assert_eq!(empty.select(), SelectionResult::First);
    }

    #[test]
    fn second_word_test() {
        let factory = WordIndexSelectorFactory::with_weights_by_index(vec![(100, 0), (0, 100)]);
        let mut empty = factory.create(CompletionParams {
            word_to_complete: "".to_string(),
            line_to_cursor: "first ".to_string(),
            line: "first ".to_string(),
        });
        assert_eq!(empty.select(), SelectionResult::Second);
    }
}
