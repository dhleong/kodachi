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
    pub line_to_cursor: String,
}

impl CompletionParams {
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            line_to_cursor: "".to_string(),
        }
    }

    #[cfg(test)]
    pub fn from_word(word: &str) -> Self {
        Self {
            line_to_cursor: word.to_string(),
        }
    }

    #[cfg(test)]
    pub fn from_line_to_cursor<T: Into<String>>(line_to_cursor: T) -> Self {
        Self {
            line_to_cursor: line_to_cursor.into(),
        }
    }

    pub fn word_to_complete(&self) -> &str {
        self.line_to_cursor.split(' ').last().unwrap_or("")
    }

    pub fn word_index(&self) -> usize {
        // NOTE: If there's a single word (with no whitespace after) that should be the 0'th index
        self.line_to_cursor.split(' ').count().saturating_sub(1)
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
            if last_word == &self.word_to_complete() {
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

    fn suggest(&self, params: CompletionParams) -> Self::Iter<'_>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_to_complete_test() {
        let params = CompletionParams::from_line_to_cursor("For the hon");
        assert_eq!(params.word_to_complete(), "hon");

        let params = CompletionParams::from_line_to_cursor("For the ");
        assert_eq!(params.word_to_complete(), "");
    }

    #[test]
    fn words_before_cursor_test() {
        let params = CompletionParams::from_line_to_cursor("For the hon");
        assert_eq!(params.words_before_cursor(), vec!["For", "the"]);
    }
}
