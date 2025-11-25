use std::collections::HashMap;

use regex::{Captures, Regex, RegexBuilder};
use serde::Deserialize;

use crate::daemon::notifications::{MatchContext, MatchedText};

use self::simple::build_simple_matcher_regex;

use super::processing::ansi::{Ansi, AnsiStripped};

pub(crate) mod simple;

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

pub struct MatchedResult {
    pub remaining: Option<Ansi>,
    pub context: MatchContext,

    // If `true` then some (possibly all) of the input was consumed.
    #[allow(unused)]
    pub consumed: bool,
}

pub enum MatchResult {
    Ignored(Ansi),

    Matched(MatchedResult),
}

#[derive(Debug)]
pub struct Matcher {
    pub options: MatcherOptions,
    pattern: Regex,
}

impl Matcher {
    pub fn try_match(&self, subject: Ansi) -> MatchResult {
        let stripped: AnsiStripped = subject.trim_trailing_newlines().strip_ansi();
        if let Some(found) = self.pattern.captures(&stripped) {
            let context = self.extract_match_context(&stripped, found);

            let mut remaining = if self.options.consume {
                subject.without_stripped_match_range(&stripped, context.full_match_range.clone())
            } else {
                subject
            };

            // If we consumed the whole line, drop any newlines
            if self.options.consume && remaining.trim().is_empty() {
                remaining = Ansi::empty();
            }

            return MatchResult::Matched(MatchedResult {
                remaining: if remaining.is_empty() {
                    None
                } else {
                    Some(remaining)
                },
                context,
                consumed: self.options.consume,
            });
        }

        MatchResult::Ignored(subject)
    }

    fn extract_match_context(&self, stripped: &AnsiStripped, captures: Captures) -> MatchContext {
        let mut named = HashMap::default();
        let mut indexed = HashMap::default();

        for i in 0..captures.len() {
            let original = stripped.get_original(captures.get(i).unwrap().range());
            indexed.insert(i, MatchedText::from(original.trim_trailing_newlines()));
        }

        for name in self.pattern.capture_names().flatten() {
            if let Some(captured) = captures.name(name) {
                let original = stripped.get_original(captured.range());
                named.insert(
                    name.to_string(),
                    MatchedText::from(original.trim_trailing_newlines()),
                );
            }
        }

        MatchContext {
            named,
            indexed,
            full_match_range: captures.get(0).unwrap().range(),
        }
    }
}

#[derive(Debug)]
pub enum MatcherCompileError {
    #[allow(unused)]
    SyntaxError(String),
    OutOfOrderIndexes,
}

impl TryInto<Matcher> for MatcherSpec {
    type Error = MatcherCompileError;

    fn try_into(self) -> Result<Matcher, Self::Error> {
        let (options, regex_source) = match self {
            MatcherSpec::Simple { options, source } => {
                (options, build_simple_matcher_regex(&source)?)
            }

            MatcherSpec::Regex { options, source } => (options, source),
        };

        match RegexBuilder::new(&regex_source).build() {
            Ok(pattern) => Ok(Matcher { options, pattern }),
            Err(e) => Err(MatcherCompileError::SyntaxError(e.to_string())),
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
        if let MatchResult::Matched(MatchedResult {
            remaining, context, ..
        }) = matcher.try_match(input.into())
        {
            assert_eq!(&remaining.unwrap()[..], "say 'anything'");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&1].plain, "anything");
        } else {
            panic!("Expected {matcher:?} to match... but it didn't");
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
        if let MatchResult::Matched(MatchedResult {
            remaining, context, ..
        }) = matcher.try_match(input.into())
        {
            assert_eq!(&remaining.unwrap()[..], "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.indexed[&0].ansi, "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.indexed[&1].ansi, "\x1b[32manything\x1b[m");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&1].plain, "anything");
        } else {
            panic!("Expected {matcher:?} to match... but it didn't");
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
        if let MatchResult::Matched(MatchedResult {
            remaining: Some(remaining),
            context,
            ..
        }) = matcher.try_match(input.into())
        {
            assert_eq!(&remaining[..], "say 'anything'");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.named[&"message".to_string()].plain, "anything");
        } else {
            panic!("Expected {matcher:?} to match... but it didn't");
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
        if let MatchResult::Matched(MatchedResult {
            remaining: Some(remaining),
            context,
            ..
        }) = matcher.try_match(input.into())
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
            panic!("Expected {matcher:?} to match... but it didn't");
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

        if let MatchResult::Matched(MatchedResult {
            remaining,
            context,
            consumed,
        }) = matcher.try_match(input.into())
        {
            assert!(consumed);
            assert_eq!(remaining, None);
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&0].ansi, "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.named[&"message".to_string()].plain, "anything");
            assert_eq!(
                &context.named[&"message".to_string()].ansi,
                "\x1b[32manything\x1b[m"
            );
        } else {
            panic!("Expected {matcher:?} to match... but it didn't");
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

        if let MatchResult::Matched(MatchedResult {
            remaining: Some(remaining),
            context,
            consumed,
        }) = matcher.try_match(input.into())
        {
            assert!(consumed);
            assert_eq!(&remaining[..], "just  you know!");
            assert_eq!(&context.indexed[&0].plain, "say 'anything'");
            assert_eq!(&context.indexed[&0].ansi, "say '\x1b[32manything\x1b[m'");
            assert_eq!(&context.named[&"message".to_string()].plain, "anything");
            assert_eq!(
                &context.named[&"message".to_string()].ansi,
                "\x1b[32manything\x1b[m"
            );
        } else {
            panic!("Expected {matcher:?} to match... but it didn't");
        }
    }

    #[test]
    fn consume_against_newlines() {
        let spec = MatcherSpec::Regex {
            options: Default::default(),
            source: "^.*honor.*$".to_string(),
        };
        let input = "For the honor of Grayskull!\r\n";

        let mut matcher: Matcher = spec.try_into().unwrap();
        matcher.options.consume = true;

        if let MatchResult::Matched(MatchedResult {
            remaining,
            context,
            consumed,
        }) = matcher.try_match(input.into())
        {
            assert!(consumed);
            assert_eq!(remaining, None);
            assert_eq!(&context.indexed[&0].plain, "For the honor of Grayskull!");
        } else {
            panic!("Expected {matcher:?} to match... but it didn't");
        }
    }

    #[test]
    fn simple() {
        let spec = MatcherSpec::Simple {
            options: Default::default(),
            source: "^wrap $burrito".to_string(),
        };
        let input = "wrap alpastor";

        let matcher: Matcher = spec.try_into().unwrap();
        if let MatchResult::Matched(MatchedResult { context, .. }) = matcher.try_match(input.into())
        {
            assert_eq!(&context.named[&"burrito".to_string()].plain, "alpastor");
        } else {
            panic!("Expected {matcher:?} to match... but it didn't");
        }
    }
}
