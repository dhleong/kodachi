use serde::Deserialize;

pub mod completions;
pub mod transforms;

#[derive(Debug, Deserialize)]
pub struct CompletionParams {
    pub word_to_complete: String,
    pub line_to_cursor: String,
    pub line: String,
}
