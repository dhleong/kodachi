use super::{CompletionParams, CompletionSource};

pub mod weighted;

pub enum SelectionResult {
    First,
    Second,
}

pub trait DuplexSelector {
    fn select(&mut self) -> SelectionResult;
}

pub trait DuplexSelectorFactory {
    type Selector: DuplexSelector;
    fn create(&self, params: CompletionParams) -> Self::Selector;
}

pub struct DuplexCompletionSource<
    A: CompletionSource,
    B: CompletionSource,
    SF: DuplexSelectorFactory,
> {
    first: A,
    second: B,
    selector: SF,
}

impl<A: CompletionSource, B: CompletionSource, SF: DuplexSelectorFactory>
    DuplexCompletionSource<A, B, SF>
{
    pub fn new(first: A, second: B, selector: SF) -> Self {
        DuplexCompletionSource {
            first,
            second,
            selector,
        }
    }
}

impl<A: CompletionSource, B: CompletionSource, SF: DuplexSelectorFactory> CompletionSource
    for DuplexCompletionSource<A, B, SF>
{
    type Iter<'a> = DuplexIter<A::Iter<'a>, B::Iter<'a>, SF::Selector> where Self: 'a;

    fn suggest<'a>(&'a self, params: super::CompletionParams) -> Self::Iter<'a> {
        DuplexIter {
            first: self.first.suggest(params.clone()),
            second: self.second.suggest(params.clone()),
            selector: self.selector.create(params),
        }
    }
}

pub struct DuplexIter<A, B, S> {
    first: A,
    second: B,
    selector: S,
}

impl<'a, A, B, S> Iterator for DuplexIter<A, B, S>
where
    A: Iterator<Item = &'a String>,
    B: Iterator<Item = &'a String>,
    S: DuplexSelector,
{
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        match self.selector.select() {
            SelectionResult::First => self.first.next().or_else(|| self.second.next()),
            SelectionResult::Second => self.second.next().or_else(|| self.first.next()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::completion::{
        duplex::weighted::WeightedRandomSelectorFactory, markov::MarkovCompletionSource,
        CompletionParams,
    };

    use super::*;

    #[test]
    fn duplex_test() {
        let mut a = MarkovCompletionSource::default();
        let mut b = MarkovCompletionSource::default();
        a.process_line("honor");
        b.process_line("grayskull");

        let duplex =
            DuplexCompletionSource::new(a, b, WeightedRandomSelectorFactory::with_weights(100, 0));
        let suggestions: Vec<&String> = duplex.suggest(CompletionParams::empty()).collect();
        assert_eq!(suggestions, vec!["honor", "grayskull"]);
    }
}
