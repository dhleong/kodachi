use std::{future::Future, io, pin::Pin};

use crate::{
    app::matchers::{MatchResult, Matcher},
    daemon::notifications::MatchContext,
};

use super::ansi::Ansi;

const MAX_RECURSION: usize = 100;

pub enum ProcessResult {
    Unchanged(String),
    ReplaceWith(String),
    Stop,
}

type MatchHandler =
    dyn Fn(MatchContext) -> Pin<Box<dyn Future<Output = io::Result<ProcessResult>>>>;
// type MatchHandler = dyn Fn(MatchContext) -> io::Result<ProcessResult>;

struct RegisteredMatcher {
    matcher: Matcher,
    on_match: Box<MatchHandler>,
}

#[derive(Default)]
pub struct SendTextProcessor {
    matchers: Vec<RegisteredMatcher>,
}

impl SendTextProcessor {
    pub fn register_matcher<
        R: 'static + Fn(MatchContext) -> Pin<Box<dyn Future<Output = io::Result<ProcessResult>>>>,
    >(
        &mut self,
        matcher: Matcher,
        on_match: R,
    ) {
        self.matchers.push(RegisteredMatcher {
            matcher,
            on_match: Box::new(on_match),
        })
    }

    pub async fn process(&self, input: String) -> io::Result<Option<String>> {
        let mut result = input;

        for _ in 0..MAX_RECURSION {
            match self.process_once(result).await? {
                ProcessResult::Unchanged(final_result) => {
                    // Nothing more to replace!
                    return Ok(Some(final_result));
                }
                ProcessResult::ReplaceWith(changed) => {
                    result = changed;
                }
                ProcessResult::Stop => {
                    return Ok(None);
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Infinite loop detected",
        ))
    }

    async fn process_once(&self, input: String) -> io::Result<ProcessResult> {
        // TODO The conversion from String <-> Ansi could be cheaper...
        let mut to_match: Ansi = input.into();
        for matcher in &self.matchers {
            match matcher.matcher.try_match(to_match) {
                MatchResult::Ignored(ignored) => to_match = ignored,
                MatchResult::Matched { context, .. } => {
                    if let Some(replaced) = self.process_match(matcher, context).await? {
                        to_match = replaced.into();
                    } else {
                        return Ok(ProcessResult::Stop);
                    }
                }
            }
        }
        Ok(ProcessResult::Unchanged(to_match.strip_ansi().to_string()))
    }

    async fn process_match(
        &self,
        matcher: &RegisteredMatcher,
        mut context: MatchContext,
    ) -> io::Result<Option<String>> {
        let result = (matcher.on_match)(context.clone());
        match result.await? {
            ProcessResult::Stop => Ok(None),
            ProcessResult::Unchanged(s) => Ok(Some(s)),
            ProcessResult::ReplaceWith(replacement) => {
                let mut input = context.take_full_match().plain;
                input.replace_range(context.full_match_range, &replacement);
                Ok(Some(input))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::matchers::MatcherSpec;

    use super::*;

    #[tokio::test]
    async fn single_replacement_test() -> io::Result<()> {
        let mut processor = SendTextProcessor::default();
        processor.register_matcher(
            MatcherSpec::Regex {
                options: Default::default(),
                source: "activate (.*)".to_string(),
            }
            .try_into()
            .unwrap(),
            |context| {
                Box::pin(async move {
                    Ok(ProcessResult::ReplaceWith(format!(
                        "yell For the Honor of Grayskull, {}!",
                        context.indexed[&1].plain
                    )))
                })
            },
        );

        let result = processor.process("activate sword".to_string()).await?;
        assert_eq!(
            result.expect("Processing was unexpectedly stopped"),
            "yell For the Honor of Grayskull, sword!"
        );

        Ok(())
    }
}