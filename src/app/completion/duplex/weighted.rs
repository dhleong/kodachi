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

pub struct WeightedRandomSelector<T: RandomnessSource + Send = ThreadRngRandomnessSource> {
    pub weights: (u8, u8),
    pub random: T,
}

impl DuplexSelector for WeightedRandomSelector {
    fn select(&mut self) -> super::SelectionResult {
        if self.random.next_percentage() <= self.weights.0 {
            SelectionResult::First
        } else {
            SelectionResult::Second
        }
    }
}

pub struct WeightedRandomSelectorFactory<
    T: 'static + RandomnessSource + Send = ThreadRngRandomnessSource,
> {
    pub weights: (u8, u8),
    pub random: T,
}

impl WeightedRandomSelectorFactory {
    pub fn with_weights(first: u8, second: u8) -> Self {
        Self {
            weights: (first, second),
            random: ThreadRngRandomnessSource,
        }
    }
}

impl DuplexSelectorFactory for WeightedRandomSelectorFactory {
    type Selector = WeightedRandomSelector;

    fn create(&self, _params: CompletionParams) -> WeightedRandomSelector {
        WeightedRandomSelector {
            weights: self.weights,
            random: self.random.clone(),
        }
    }
}
