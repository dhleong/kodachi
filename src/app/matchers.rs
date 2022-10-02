use std::collections::HashMap;

use regex::{Captures, Regex};
use serde::Deserialize;

use crate::daemon::notifications::{MatchContext, MatchedText};

use super::processing::ansi::{Ansi, AnsiStripped};

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

        // If `true` then some (possibly all) of the input was consumed.
        consumed: bool,
    },
}

#[derive(Debug)]
pub struct Matcher {
    options: MatcherOptions,
    pattern: Regex,
}

impl Matcher {
    pub fn try_match(&self, mut subject: Ansi) -> MatchResult {
        let stripped = subject.strip_ansi();
        if let Some(found) = self.pattern.captures(&stripped) {
            let context = self.extract_match_context(&stripped, found);

            let remaining = if self.options.consume {
                let consumed_range = stripped.get_original_range(context.full_match_range.clone());
                subject.slice(0..consumed_range.start)
                    + subject.slice(consumed_range.end..subject.len())
            } else {
                subject
            };

            return MatchResult::Matched {
                remaining,
                context,
                consumed: self.options.consume,
            };
        }

        MatchResult::Ignored(subject)
    }

    fn extract_match_context(&self, stripped: &AnsiStripped, captures: Captures) -> MatchContext {
        let mut named = HashMap::default();
        let mut indexed = HashMap::default();

        for i in 0..captures.len() {
            let original = stripped.get_original(captures.get(i).unwrap().range());
            indexed.insert(i, MatchedText::from(original));
        }

        for name in self.pattern.capture_names() {
            if let Some(n) = name {
                if let Some(captured) = captures.name(n) {
                    let original = stripped.get_original(captured.range());
                    named.insert(n.to_string(), MatchedText::from(original));
                }
            }
        }

        return MatchContext {
            named,
            indexed,
            full_match_range: captures.get(0).unwrap().range(),
        };
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
        if let MatchResult::Matched {
            remaining, context, ..
        } = matcher.try_match(input.into())
        {
            assert_eq!(&remaining[..], "say 'anything'");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&1].plain, "anything");
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }

    #[test]
    fn indexed_ansi_matches_test() {
        let spec = MatcherSpec::Regex {
            options: Default::default(),
            source: "say '(.*)'".to_string(),
        };
        let input = "say '\x1b[32manything\x1b[m'";

        let matcher: Matcher = spec.try_into().unwrap();
        if let MatchResult::Matched {
            remaining, context, ..
        } = matcher.try_match(input.into())
        {
            assert_eq!(&remaining[..], "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.indexed[&0].ansi, "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.indexed[&1].ansi, "\x1b[32manything\x1b[m");
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
        if let MatchResult::Matched {
            remaining, context, ..
        } = matcher.try_match(input.into())
        {
            assert_eq!(&remaining[..], "say 'anything'");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.named[&"message".to_string()].plain, "anything");
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }

    #[test]
    fn named_ansi_matches_test() {
        let spec = MatcherSpec::Regex {
            options: Default::default(),
            source: "say '(?P<message>.*)'".to_string(),
        };
        let input = "say '\x1b[32manything\x1b[m'";

        let matcher: Matcher = spec.try_into().unwrap();
        if let MatchResult::Matched {
            remaining, context, ..
        } = matcher.try_match(input.into())
        {
            assert_eq!(&remaining[..], "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&0].ansi, "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.named[&"message".to_string()].plain, "anything");
            assert_eq!(
                &context.named[&"message".to_string()].ansi,
                "\x1b[32manything\x1b[m"
            );
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }

    #[test]
    fn consume_full_line() {
        let spec = MatcherSpec::Regex {
            options: Default::default(),
            source: "say '(?P<message>.*)'".to_string(),
        };
        let input = "say '\x1b[32manything\x1b[m'";

        let mut matcher: Matcher = spec.try_into().unwrap();
        matcher.options.consume = true;

        if let MatchResult::Matched {
            remaining,
            context,
            consumed,
        } = matcher.try_match(input.into())
        {
            assert_eq!(consumed, true);
            assert_eq!(&remaining[..], "");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&0].ansi, "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.named[&"message".to_string()].plain, "anything");
            assert_eq!(
                &context.named[&"message".to_string()].ansi,
                "\x1b[32manything\x1b[m"
            );
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }

    #[test]
    fn consume_within_line() {
        let spec = MatcherSpec::Regex {
            options: Default::default(),
            source: "say '(?P<message>.*)'".to_string(),
        };
        let input = "just say '\x1b[32manything\x1b[m' you know!";

        let mut matcher: Matcher = spec.try_into().unwrap();
        matcher.options.consume = true;

        if let MatchResult::Matched {
            remaining,
            context,
            consumed,
        } = matcher.try_match(input.into())
        {
            assert_eq!(consumed, true);
            assert_eq!(&remaining[..], "just  you know!");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&0].ansi, "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.named[&"message".to_string()].plain, "anything");
            assert_eq!(
                &context.named[&"message".to_string()].ansi,
                "\x1b[32manything\x1b[m"
            );
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }
}
