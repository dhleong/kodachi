use serde::Deserialize;

use super::processing::ansi::Ansi;

#[derive(Debug, Deserialize)]
pub struct MatcherOptions {
    #[serde(default)]
    pub consume: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum MatcherSpec {
    Regex {
        #[serde(flatten)]
        options: MatcherOptions,
        source: String,
    },
    Simple {
        #[serde(flatten)]
        options: MatcherOptions,
        source: String,
    },
}

pub enum MatchResult {
    Ignored(Ansi),

    /// Some (or all) of the input was consumed; [remaining] contains any remaining text
    Consumed {
        remaining: Ansi,
    },
}
