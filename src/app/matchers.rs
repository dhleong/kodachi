use std::collections::HashMap;

use regex::{Captures, Regex, RegexBuilder};
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
    pub options: MatcherOptions,
    pattern: Regex,
}

impl Matcher {
    pub fn try_match(&self, mut subject: Ansi) -> MatchResult {
        let stripped = subject.strip_ansi();
        if let Some(found) = self.pattern.captures(&stripped) {
            let context = self.extract_match_context(&stripped, found);

            let mut remaining = if self.options.consume {
                let consumed_range = stripped.get_original_range(context.full_match_range.clone());
                subject.slice(0..consumed_range.start)
                    + subject.slice(consumed_range.end..subject.len())
            } else {
                subject
            };

            // If we consumed the whole line, drop any newlines
            if self.options.consume && remaining.trim().is_empty() {
                remaining = Ansi::empty();
            }

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
            indexed.insert(i, MatchedText::from(original.trim_trailing_newlines()));
        }

        for name in self.pattern.capture_names() {
            if let Some(n) = name {
                if let Some(captured) = captures.name(n) {
                    let original = stripped.get_original(captured.range());
                    named.insert(
                        n.to_string(),
                        MatchedText::from(original.trim_trailing_newlines()),
                    );
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
    OutOfOrderIndexes,
}

fn build_simple_matcher_regex(mut source: &str) -> Result<String, MatcherCompileError> {
    let mut pattern = String::new();
    let var_regex = Regex::new(r"\$(\d+|\w+|(?:\{(\w+)\}))").unwrap();

    // Special case to bind to start-of-line
    if source.get(0..1) == Some("^") {
        source = &source[1..];
        pattern.push('^');
    }

    let mut last_var_end = 0;
    let mut last_index: Option<usize> = None;
    for capture in var_regex.captures_iter(source).into_iter() {
        let range = capture.get(0).unwrap().range();
        let start = range.start;
        if start > 0 && &source[start - 1..start] == "$" {
            // Escaped variable; keep moving. Normally we might use a negative lookbehind
            // assertion for this, but the regex crate doesn't support those, so we do it
            // manually here.
            continue;
        }

        if start > last_var_end {
            pattern.push_str(&regex::escape(&source[last_var_end..start]));
        }

        let var = capture.get(1).unwrap();
        if let Ok(as_index) = var.as_str().parse::<usize>() {
            if let Some(last_index) = last_index {
                if as_index <= last_index {
                    return Err(MatcherCompileError::OutOfOrderIndexes);
                }
            }
            last_index = Some(as_index);

            pattern.push_str("(.+)");
        } else {
            pattern.push_str("(?<");
            pattern.push_str(var.as_str());
            pattern.push_str(">.+)");
        }

        last_var_end = range.end;
    }

    if last_var_end < source.len() {
        pattern.push_str(&regex::escape(&source[last_var_end..]));
    }

    return Ok(pattern);
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

        match RegexBuilder::new(&regex_source).multi_line(true).build() {
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

    #[test]
    fn consume_against_newlines() {
        let spec = MatcherSpec::Regex {
            options: Default::default(),
            source: "^.*honor.*$".to_string(),
        };
        let input = "For the honor of Grayskull!\r\n";

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
            assert_eq!(&context.indexed[&0].plain, "For the honor of Grayskull!");
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }

    #[cfg(test)]
    mod simple_matcher_tests {
        use super::*;

        #[test]
        fn build_indexed_pattern_test() {
            let pattern = build_simple_matcher_regex("$1 {activate} $2 [now]").unwrap();
            assert_eq!(pattern, r"(.+) \{activate\} (.+) \[now\]");
        }

        #[test]
        fn build_named_pattern_test() {
            let pattern = build_simple_matcher_regex("$first {activate} $second [now]").unwrap();
            assert_eq!(pattern, r"(?<first>.+) \{activate\} (?<second>.+) \[now\]");
        }

        #[test]
        fn accept_line_start_test() {
            let pattern = build_simple_matcher_regex("^admire $thing").unwrap();
            assert_eq!(pattern, r"^admire (?<thing>.+)");
        }
    }
}
