use rand::Rng;

use crate::app::completion::CompletionParams;

use super::{DuplexSelector, DuplexSelectorFactory, SelectionResult};

pub trait RandomnessSource: Clone {
    fn next_percentage(&mut self) -> u8;
}

#[derive(Clone)]
pub struct ThreadRngRandomnessSource;
impl RandomnessSource for ThreadRngRandomnessSource {
    fn next_percentage(&mut self) -> u8 {
        rand::thread_rng().gen_range(0..=100)
    }
}

pub struct WeightedRandomSelector<R: RandomnessSource + Send = ThreadRngRandomnessSource> {
    pub weights: (u8, u8),
    pub random: R,
}

impl<R: RandomnessSource + Send> DuplexSelector for WeightedRandomSelector<R> {
    fn select(&mut self) -> super::SelectionResult {
        if self.random.next_percentage() <= self.weights.0 {
            SelectionResult::First
        } else {
            SelectionResult::Second
        }
    }
}

pub struct WeightedRandomSelectorFactory<
    R: 'static + RandomnessSource + Send = ThreadRngRandomnessSource,
> {
    pub weights: (u8, u8),
    pub random: R,
}

impl WeightedRandomSelectorFactory {
    pub fn with_weights(first: u8, second: u8) -> Self {
        if first + second != 100 {
            panic!(
                "Weights must sum to 100; received {:?}, {:?}",
                first, second
            );
        }
        Self {
            weights: (first, second),
            random: ThreadRngRandomnessSource,
        }
    }

    pub fn with_random<R: RandomnessSource + Send + 'static>(
        self,
        random: R,
    ) -> WeightedRandomSelectorFactory<R> {
        WeightedRandomSelectorFactory {
            weights: self.weights,
            random,
        }
    }
}

impl<R: RandomnessSource + Send> DuplexSelectorFactory for WeightedRandomSelectorFactory<R> {
    type Selector = WeightedRandomSelector<R>;

    fn create(&self, _params: CompletionParams) -> Self::Selector {
        WeightedRandomSelector {
            weights: self.weights,
            random: self.random.clone(),
        }
    }
}

#[cfg(test)]
#[derive(Clone)]
pub struct StaticRandomnessSource {
    values: Vec<u8>,
}
#[cfg(test)]
impl StaticRandomnessSource {
    pub fn with_values(values: Vec<u8>) -> Self {
        Self { values }
    }
}

#[cfg(test)]
impl RandomnessSource for StaticRandomnessSource {
    fn next_percentage(&mut self) -> u8 {
        if self.values.is_empty() {
            0
        } else {
            self.values.remove(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn weighted_selection() {
        let random = StaticRandomnessSource::with_values(vec![59, 61, 42, 20, 2]);
        let mut selector = WeightedRandomSelectorFactory {
            weights: (60, 40),
            random,
        }
        .create(CompletionParams::empty());

        // 0.59 - below 60 should go to first source
        assert_eq!(selector.select(), SelectionResult::First);

        // 0.61 - above 60 should go to second source
        assert_eq!(selector.select(), SelectionResult::Second);
    }
}
