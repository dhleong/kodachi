use std::{future::Future, io, pin::Pin};

use async_trait::async_trait;

use crate::{
    app::matchers::{MatchResult, Matcher},
    daemon::{
        notifications::{DaemonNotification, MatchContext},
        responses::DaemonResponse,
    },
};

use super::ansi::Ansi;

const MAX_RECURSION: usize = 100;

pub enum ProcessResult {
    Unchanged(String),
    ReplaceWith(String),
    Stop,
}

type MatchHandler = dyn Fn(MatchContext) -> Pin<Box<dyn Future<Output = io::Result<ProcessResult>> + Send + Sync>>
    + Send
    + Sync;

struct RegisteredMatcher {
    matcher: Matcher,
    on_match: Box<MatchHandler>,
}

#[async_trait]
pub trait SendTextProcessorOutputReceiver {
    async fn request(&mut self, request: DaemonNotification) -> io::Result<DaemonResponse>;
}

#[derive(Default)]
pub struct SendTextProcessor {
    matchers: Vec<RegisteredMatcher>,
}

impl SendTextProcessor {
    pub fn register_matcher<R, F>(&mut self, matcher: Matcher, on_match: R)
    where
        R: 'static + (Fn(MatchContext) -> F) + Send + Sync,
        F: 'static + Future<Output = io::Result<ProcessResult>> + Send + Sync,
    {
        if matcher.options.consume {
            panic!("Matcher ({:?}) is unexpectedly `consume`", matcher);
        }

        self.matchers.push(RegisteredMatcher {
            matcher,
            on_match: Box::new(move |context| Box::pin(on_match(context))),
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
        // TODO The conversion from String <-> Ansi could be cheaper... Or perhaps
        // we could refactor to use a trait instead of Ansi?
        let mut to_match: Ansi = input.clone().into();
        let mut unchanged = true;
        for matcher in &self.matchers {
            match matcher.matcher.try_match(to_match) {
                MatchResult::Ignored(ignored) => to_match = ignored,
                MatchResult::Matched {
                    context,
                    mut remaining,
                    ..
                } => {
                    // NOTE: Since the matcher *shouldn't* consume, remaining *should*
                    // be the original input
                    if let Some(replaced) = self
                        .process_match(matcher, context, remaining.strip_ansi().to_string())
                        .await?
                    {
                        unchanged = false;
                        to_match = replaced.into();
                    } else {
                        return Ok(ProcessResult::Stop);
                    }
                }
            }
        }

        if unchanged {
            Ok(ProcessResult::Unchanged(input))
        } else {
            Ok(ProcessResult::ReplaceWith(
                to_match.strip_ansi().to_string(),
            ))
        }
    }

    async fn process_match(
        &self,
        matcher: &RegisteredMatcher,
        context: MatchContext,
        mut original: String,
    ) -> io::Result<Option<String>> {
        let result = (matcher.on_match)(context.clone());
        match result.await? {
            ProcessResult::Stop => Ok(None),
            ProcessResult::Unchanged(s) => Ok(Some(s)),
            ProcessResult::ReplaceWith(replacement) => {
                original.replace_range(context.full_match_range, &replacement);
                Ok(Some(original))
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
            |context| async move {
                Ok(ProcessResult::ReplaceWith(format!(
                    "yell For the Honor of Grayskull, {}!",
                    context.indexed[&1].plain
                )))
            },
        );

        let result = processor.process("activate sword".to_string()).await?;
        assert_eq!(
            result.expect("Processing was unexpectedly stopped"),
            "yell For the Honor of Grayskull, sword!"
        );

        Ok(())
    }

    #[tokio::test]
    async fn non_start_replacement_test() -> io::Result<()> {
        let mut processor = SendTextProcessor::default();
        processor.register_matcher(
            MatcherSpec::Regex {
                options: Default::default(),
                source: "honor (.*)".to_string(),
            }
            .try_into()
            .unwrap(),
            |context| async move {
                Ok(ProcessResult::ReplaceWith(format!(
                    "yell For the Honor of {}!",
                    context.indexed[&1].plain
                )))
            },
        );

        let result = processor
            .process("Let's honor Grayskull".to_string())
            .await?;
        assert_eq!(
            result.expect("Processing was unexpectedly stopped"),
            "Let's yell For the Honor of Grayskull!"
        );

        Ok(())
    }

    #[tokio::test]
    async fn detect_recursion_test() -> io::Result<()> {
        let mut processor = SendTextProcessor::default();
        processor.register_matcher(
            MatcherSpec::Regex {
                options: Default::default(),
                source: "honor (.*)".to_string(),
            }
            .try_into()
            .unwrap(),
            |context| async move {
                Ok(ProcessResult::ReplaceWith(format!(
                    "yell for the honor of {}!",
                    context.indexed[&1].plain
                )))
            },
        );

        let result = processor.process("honor Grayskull".to_string()).await;
        let err = result.expect_err("Expected a recursion error");
        assert!(err.to_string().contains("Infinite loop"));

        Ok(())
    }

    #[tokio::test]
    async fn multi_replacement_test() -> io::Result<()> {
        let mut processor = SendTextProcessor::default();
        processor.register_matcher(
            MatcherSpec::Regex {
                options: Default::default(),
                source: "^honor (.*)".to_string(),
            }
            .try_into()
            .unwrap(),
            |context| async move {
                Ok(ProcessResult::ReplaceWith(format!(
                    "yell For the honor of {}!",
                    context.indexed[&1].plain
                )))
            },
        );

        processor.register_matcher(
            MatcherSpec::Regex {
                options: Default::default(),
                source: "honor of ([a-z]+)".to_string(),
            }
            .try_into()
            .unwrap(),
            |context| async move {
                Ok(ProcessResult::ReplaceWith(format!(
                    "Honor of {}!",
                    context.indexed[&1].plain.to_uppercase()
                )))
            },
        );

        let result = processor.process("honor grayskull".to_string()).await?;
        assert_eq!(
            result.expect("Processing was unexpectedly stopped"),
            "yell For the Honor of GRAYSKULL!!"
        );

        Ok(())
    }
}
