use serde::Deserialize;

pub mod completions;
pub mod duplex;
mod filtering;
pub mod markov;
pub mod recency;
mod sent;
pub mod transforms;

#[derive(Clone, Debug, Deserialize)]
pub struct CompletionParams {
    pub word_to_complete: String,
    pub line_to_cursor: String,
    #[allow(unused)] // useful for debug
    pub line: String,
}

impl CompletionParams {
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            word_to_complete: "".to_string(),
            line_to_cursor: "".to_string(),
            line: "".to_string(),
        }
    }

    #[cfg(test)]
    pub fn from_word(word: &str) -> Self {
        Self {
            word_to_complete: word.to_string(),
            line_to_cursor: word.to_string(),
            line: word.to_string(),
        }
    }

    pub fn word_index(&self) -> usize {
        // NOTE: If there's a single word (with no whitespace after) that should be the 0'th index
        self.line_to_cursor
            .split(' ')
            .count()
            .checked_sub(1)
            .unwrap_or(0)
    }

    /// Return a vector of the words before the cursor, NOT inclusive if any partial word
    /// directly touching the cursor.
    ///
    /// In other words, for the input:
    ///
    ///     For the hon|
    ///                ^-- cursor
    ///
    /// This method will return `vec!["For", "the"]`.
    pub fn words_before_cursor(&self) -> Vec<&str> {
        let mut words: Vec<&str> = self.line_to_cursor.split_whitespace().collect();
        if let Some(last_word) = words.last() {
            if last_word == &self.word_to_complete {
                words.pop();
            }
        }
        return words;
    }
}

pub trait CompletionSource {
    type Iter<'a>: Iterator<Item = &'a String>
    where
        Self: 'a;

    fn suggest<'a>(&'a self, params: CompletionParams) -> Self::Iter<'a>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_before_cursor_test() {
        let params = CompletionParams {
            word_to_complete: "hon".to_string(),
            line_to_cursor: "For the hon".to_string(),
            line: "For the hon".to_string(),
        };
        assert_eq!(params.words_before_cursor(), vec!["For", "the"]);
    }
}
