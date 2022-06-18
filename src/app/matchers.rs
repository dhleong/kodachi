use std::collections::HashMap;

use regex::{Captures, Regex};
use serde::Deserialize;

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

#[derive(PartialEq, Eq, Hash)]
pub enum MatchVariable {
    Index(usize),
    Name(String),
}

pub enum MatchResult {
    Ignored(Ansi),

    Matched {
        remaining: Ansi,
        full_match: Ansi,
        variables: HashMap<MatchVariable, Ansi>,
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
            let full_match = Ansi::from(found.get(0).unwrap().as_str());
            let remaining = subject.clone();

            let variables = self.extract_match_variables(found);

            return MatchResult::Matched {
                remaining,
                full_match,
                variables,
            };
        }

        println!("pattern {:?} did not find", self.pattern);
        MatchResult::Ignored(subject)
    }

    fn extract_match_variables(&self, captures: Captures) -> HashMap<MatchVariable, Ansi> {
        let mut result = HashMap::default();

        for i in 0..captures.len() {
            result.insert(
                MatchVariable::Index(i),
                Ansi::from(captures.get(i).unwrap().as_str()),
            );
        }

        for name in self.pattern.capture_names() {
            if let Some(n) = name {
                if let Some(captured) = captures.name(n) {
                    result.insert(
                        MatchVariable::Name(n.to_string()),
                        Ansi::from(captured.as_str()),
                    );
                }
            }
        }

        return result;
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
            remaining,
            full_match,
            variables,
        } = matcher.try_match(input.into())
        {
            assert_eq!(&remaining[..], "say 'anything'");
            assert_eq!(&full_match[..], "say 'anything'");
            assert_eq!(&(variables[&MatchVariable::Index(0)])[..], "say 'anything'");
            assert_eq!(&(variables[&MatchVariable::Index(1)])[..], "anything");
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
            remaining,
            full_match,
            variables,
        } = matcher.try_match(input.into())
        {
            assert_eq!(&remaining[..], "say 'anything'");
            assert_eq!(&full_match[..], "say 'anything'");
            assert_eq!(&(variables[&MatchVariable::Index(0)])[..], "say 'anything'");
            assert_eq!(
                &(variables[&MatchVariable::Name("message".to_string())])[..],
                "anything"
            );
        } else {
            panic!("Expected {:?} to match... but it didn't", matcher);
        }
    }
}
