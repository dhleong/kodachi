use serde::Deserialize;

pub mod completions;
pub mod markov;
pub mod transforms;

#[derive(Debug, Deserialize)]
pub struct CompletionParams {
    pub word_to_complete: String,
    pub line_to_cursor: String,
    pub line: String,
}

impl CompletionParams {
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
