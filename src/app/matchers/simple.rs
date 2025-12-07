use std::ops::Range;

use lazy_static::lazy_static;

use regex::{Captures, Regex};

use super::MatcherCompileError;

lazy_static! {
    pub static ref VAR_REGEX: Regex = Regex::new(r"\$(\d+|\w+|(?:\{(\w+)\}))").unwrap();
}

pub enum VarLabel<'a> {
    Index(usize),
    Name(&'a str),
}

pub struct SimpleVar<'a> {
    range: Range<usize>,
    pub label: VarLabel<'a>,
}

pub fn unpack_var<'a>(source: &str, capture: &'a Captures) -> Option<SimpleVar<'a>> {
    let range = capture.get(0).unwrap().range();
    let start = range.start;
    if start > 0 && &source[start - 1..start] == "$" {
        // Escaped variable; keep moving. Normally we might use a negative lookbehind
        // assertion for this, but the regex crate doesn't support those, so we do it
        // manually here.
        return None;
    }

    let var = capture.get(1).unwrap();
    let label = if let Ok(as_index) = var.as_str().parse::<usize>() {
        VarLabel::Index(as_index)
    } else {
        let mut var_name = var.as_str();
        if var_name.starts_with("{") {
            // Strip the disambiguating brackets
            var_name = &var_name[1..var_name.len() - 1];
        }

        VarLabel::Name(var_name)
    };

    return Some(SimpleVar { range, label });
}

pub fn build_simple_matcher_regex(mut source: &str) -> Result<String, MatcherCompileError> {
    let mut pattern = String::new();

    // Special case to bind to start-of-line
    if source.get(0..1) == Some("^") {
        source = &source[1..];
        pattern.push('^');
    }

    let mut last_var_end = 0;
    let mut last_index: Option<usize> = None;
    for capture in VAR_REGEX.captures_iter(source) {
        let var = if let Some(var) = unpack_var(source, &capture) {
            var
        } else {
            continue;
        };

        if var.range.start > last_var_end {
            pattern.push_str(&regex::escape(&source[last_var_end..var.range.start]));
        }

        match var.label {
            VarLabel::Index(as_index) => {
                if let Some(last_index) = last_index {
                    if as_index <= last_index {
                        return Err(MatcherCompileError::OutOfOrderIndexes);
                    }
                }
                last_index = Some(as_index);

                pattern.push_str("(.+)");
            }
            VarLabel::Name(as_name) => {
                pattern.push_str("(?P<");
                pattern.push_str(as_name);
                pattern.push_str(">.+)");
            }
        }

        last_var_end = var.range.end;
    }

    if last_var_end < source.len() {
        pattern.push_str(&regex::escape(&source[last_var_end..]));
    }

    return Ok(pattern);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_indexed_pattern_test() {
        let pattern = build_simple_matcher_regex("$1 {activate} $2 [now]").unwrap();
        assert_eq!(pattern, r"(.+) \{activate\} (.+) \[now\]");
    }

    #[test]
    fn build_named_pattern_test() {
        let pattern = build_simple_matcher_regex("$first {activate} $second [now]").unwrap();
        assert_eq!(
            pattern,
            r"(?P<first>.+) \{activate\} (?P<second>.+) \[now\]"
        );
    }

    #[test]
    fn build_disambiguated_named_pattern_test() {
        let pattern = build_simple_matcher_regex("${first}and${second}").unwrap();
        assert_eq!(pattern, r"(?P<first>.+)and(?P<second>.+)");
    }

    #[test]
    fn accept_line_start_test() {
        let pattern = build_simple_matcher_regex("^admire $thing").unwrap();
        assert_eq!(pattern, r"^admire (?P<thing>.+)");
    }
}
