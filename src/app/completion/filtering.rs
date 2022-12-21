use super::{CompletionParams, CompletionSource};

pub struct FilteringCompletionSource<S>(pub S);

impl<S: CompletionSource> CompletionSource for FilteringCompletionSource<S> {
    type Iter<'a> = Filtered<S::Iter<'a>>
    where
        Self: 'a;

    fn suggest<'a>(&'a self, params: CompletionParams) -> Self::Iter<'a> {
        Filtered(self.0.suggest(params.clone()), params)
    }
}

pub struct Filtered<I>(I, CompletionParams);

impl<'a, I: Iterator<Item = &'a String>> Iterator for Filtered<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.find(|item| candidate_matches_params(&self.1, item))
    }
}

fn candidate_matches_params(params: &CompletionParams, candidate: &str) -> bool {
    let mut chars_to_include = params.word_to_complete.chars();
    let mut candidate_chars = candidate.chars();
    'outer: loop {
        if let Some(next_char) = chars_to_include.next() {
            for ch in &mut candidate_chars {
                if ch.eq_ignore_ascii_case(&next_char) {
                    continue 'outer;
                }
            }

            // If we got here, there are no more chars in the candidate, but
            // we did not match a required character in the word_to_complete;
            // this candidate must not match
            return false;
        } else {
            // No more chars in the word_to_complete; we're good to go
            return true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_params_accept(params: &CompletionParams, candidate: &str, should_accept: bool) {
        let did_accept = candidate_matches_params(params, candidate);
        if did_accept != should_accept {
            let verb = if should_accept { "accept" } else { "reject" };
            assert!(
                false,
                "Expected {:?} to {} {:?} but it did not",
                params, verb, candidate
            );
        }
    }

    #[test]
    fn empty_params_test() {
        let params = CompletionParams::empty();
        assert_params_accept(&params, "alpastor", true);
        assert_params_accept(&params, "andpinto", true);
    }

    #[test]
    fn ordered_filtering_test() {
        let params = CompletionParams::from_word("ap");
        assert_params_accept(&params, "alpastor", true);
        assert_params_accept(&params, "andpinto", true);

        // characters matched, but out of order
        assert_params_accept(&params, "plus ultra", false);
    }

    #[test]
    fn case_insensitivity() {
        let params = CompletionParams::from_word("Ap");
        assert_params_accept(&params, "alpastor", true);
        assert_params_accept(&params, "andpinto", true);

        // characters matched, but out of order
        assert_params_accept(&params, "plus ultra", false);
    }
}
