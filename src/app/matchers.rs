use std::collections::HashMap;

use regex::{Captures, Regex};
use serde::Deserialize;

use crate::daemon::notifications::{MatchContext, MatchedText};

use super::processing::ansi::Ansi;

#[derive(Debug, Default, Deserialize)]
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

    Matched {
        remaining: Ansi,
        context: MatchContext,
    },

    /// Some (or all) of the input was consumed; [remaining] contains any remaining text
    Consumed {
        remaining: Ansi,
    },
}

#[derive(Debug)]
pub struct Matcher {
    options: MatcherOptions,
    pattern: Regex,
}

impl Matcher {
    pub fn try_match(&self, subject: Ansi) -> MatchResult {
        if let Some(found) = self.pattern.captures(&subject) {
            if self.options.consume {
                // TODO: Consume
                return MatchResult::Consumed { remaining: subject };
            }

            // TODO: Map pattern range back to Ansi bytes range
            let remaining = subject.clone();
            let context = self.extract_match_context(found);

            return MatchResult::Matched { remaining, context };
        }

        println!("pattern {:?} did not find", self.pattern);
        MatchResult::Ignored(subject)
    }

    fn extract_match_context(&self, captures: Captures) -> MatchContext {
        let mut named = HashMap::default();
        let mut indexed = HashMap::default();

        for i in 0..captures.len() {
            indexed.insert(
                i,
                MatchedText::from_raw_ansi(captures.get(i).unwrap().as_str()),
            );
        }

        for name in self.pattern.capture_names() {
            if let Some(n) = name {
                if let Some(captured) = captures.name(n) {
                    named.insert(n.to_string(), MatchedText::from_raw_ansi(captured.as_str()));
                }
            }
        }

        return MatchContext { named, indexed };
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
            MatcherSpec::Regex { options, source } => match Regex::new(&source) {
                Ok(pattern) => Ok(Matcher { options, pattern }),
                Err(e) => Err(MatcherCompileError::SyntaxError(e.to_string())),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_matches_test() {
        let spec = MatcherSpec::Regex {
            options: Default::default(),
            source: "say '(.*)'".to_string(),
        };
        let input = "say 'anything'";

        let matcher: Matcher = spec.try_into().unwrap();
        if let MatchResult::Matched { remaining, context } = matcher.try_match(input.into()) {
            assert_eq!(&remaining[..], "say 'anything'");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&1].plain, "anything");
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }

    #[test]
    fn named_matches_test() {
        let spec = MatcherSpec::Regex {
            options: Default::default(),
            source: "say '(?P<message>.*)'".to_string(),
        };
        let input = "say 'anything'";

        let matcher: Matcher = spec.try_into().unwrap();
        if let MatchResult::Matched { remaining, context } = matcher.try_match(input.into()) {
            assert_eq!(&remaining[..], "say 'anything'");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.named[&"message".to_string()].plain, "anything");
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }
}
