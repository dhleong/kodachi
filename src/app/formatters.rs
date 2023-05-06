use regex::Captures;
use serde::Deserialize;

use crate::daemon::notifications::MatchContext;

use super::matchers::{
    simple::{build_simple_matcher_regex, unpack_var, VarLabel, VAR_REGEX},
    MatcherCompileError,
};

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum FormatterSpec {
    Simple(String),
}

pub struct Formatter {
    source: String,
}

impl Formatter {
    pub fn format(&self, context: MatchContext) -> String {
        VAR_REGEX
            .replace_all(&self.source, |captures: &Captures| {
                if let Some(var) = unpack_var(&self.source, captures) {
                    match var.label {
                        VarLabel::Index(index) => context
                            .indexed
                            .get(&index)
                            .map_or("".to_string(), |m| m.plain.to_string()),
                        VarLabel::Name(name) => context
                            .named
                            .get(name)
                            .map_or("".to_string(), |m| m.plain.to_string()),
                    }
                } else {
                    // Not a var? Pass through as-is (dropping the leading $)
                    captures.get(1).unwrap().as_str().to_string()
                }
            })
            .to_string()
    }
}

impl TryInto<Formatter> for FormatterSpec {
    type Error = MatcherCompileError;

    fn try_into(self) -> Result<Formatter, Self::Error> {
        let source = match self {
            FormatterSpec::Simple(source) => source,
        };

        build_simple_matcher_regex(&source)?;

        Ok(Formatter { source })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::daemon::notifications::MatchedText;

    use super::*;

    #[test]
    fn format_indexed_test() {
        let pattern = "activate $1";
        let formatter: Formatter = FormatterSpec::Simple(pattern.to_string())
            .try_into()
            .unwrap();
        let formatted = formatter.format(MatchContext {
            named: Default::default(),
            indexed: HashMap::from([(1, MatchedText::from("Grayskull".into()))]),
            full_match_range: 0..1,
        });
        assert_eq!(formatted, "activate Grayskull");
    }

    #[test]
    fn format_non_vars() {
        let pattern = "give $$3.50";
        let formatter: Formatter = FormatterSpec::Simple(pattern.to_string())
            .try_into()
            .unwrap();
        let formatted = formatter.format(MatchContext {
            named: Default::default(),
            indexed: Default::default(),
            full_match_range: 0..1,
        });
        assert_eq!(formatted, "give $3.50");
    }

    #[test]
    fn format_names_test() {
        let pattern = "honor ${color}$thing";
        let formatter: Formatter = FormatterSpec::Simple(pattern.to_string())
            .try_into()
            .unwrap();
        let formatted = formatter.format(MatchContext {
            named: HashMap::from([
                ("color".to_string(), MatchedText::from("Gray".into())),
                ("thing".to_string(), MatchedText::from("skull".into())),
            ]),
            indexed: Default::default(),
            full_match_range: 0..1,
        });
        assert_eq!(formatted, "honor Grayskull");
    }
}
