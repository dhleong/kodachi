use std::str::FromStr;

use regex::Regex;
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

pub struct Matcher {
    options: MatcherOptions,
    pattern: Regex,
}

impl Matcher {
    pub fn try_match(&self, subject: Ansi) -> MatchResult {
        if let Some(_found) = self.pattern.find(&subject) {
            if self.options.consume {
                // TODO: Consume
                return MatchResult::Consumed { remaining: subject };
            }

            // TODO extract vars from "found"
        }
        MatchResult::Ignored(subject)
    }
}

#[derive(Debug)]
pub enum MatcherCompileError {
    SyntaxError(String),

    /// TODO
    Unsupported,
}

impl TryInto<Matcher> for MatcherSpec {
    type Error = MatcherCompileError;

    fn try_into(self) -> Result<Matcher, Self::Error> {
        match self {
            MatcherSpec::Simple { .. } => Err(MatcherCompileError::Unsupported),
            MatcherSpec::Regex { options, source } => match Regex::from_str(&source) {
                Ok(pattern) => Ok(Matcher { options, pattern }),
                Err(e) => Err(MatcherCompileError::SyntaxError(e.to_string())),
            },
        }
    }
}
