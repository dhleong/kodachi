use super::CompletionSource;

pub struct DuplexCompletionSource<A: CompletionSource, B: CompletionSource> {
    first: A,
    second: B,
}

impl<A: CompletionSource, B: CompletionSource> DuplexCompletionSource<A, B> {
    pub fn new(first: A, second: B) -> Self {
        DuplexCompletionSource { first, second }
    }
}

impl<A: CompletionSource, B: CompletionSource> CompletionSource for DuplexCompletionSource<A, B> {
    type Iter<'a> = DuplexIter<A::Iter<'a>, B::Iter<'a>> where Self: 'a;

    fn suggest<'a>(&'a self, params: super::CompletionParams) -> Self::Iter<'a> {
        DuplexIter {
            first: self.first.suggest(params.clone()),
            second: self.second.suggest(params),
        }
    }
}

pub struct DuplexIter<A, B> {
    first: A,
    second: B,
}

impl<'a, A, B> Iterator for DuplexIter<A, B>
where
    A: Iterator<Item = &'a String>,
    B: Iterator<Item = &'a String>,
{
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: selection heuristic
        if let Some(from_a) = self.first.next() {
            Some(from_a)
        } else {
            self.second.next()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::completion::{markov::MarkovCompletionSource, CompletionParams};

    use super::*;

    #[test]
    fn duplex_test() {
        let mut a = MarkovCompletionSource::default();
        let mut b = MarkovCompletionSource::default();
        a.process_line("honor");
        b.process_line("grayskull");

        let duplex = DuplexCompletionSource::new(a, b);
        let suggestions: Vec<&String> = duplex.suggest(CompletionParams::empty()).collect();
        assert_eq!(suggestions, vec!["honor", "grayskull"]);
    }
}
