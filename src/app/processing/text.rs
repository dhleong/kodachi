use std::{
    io,
    sync::{Arc, Mutex},
};

use bytes::Buf;

use crate::{
    app::{
        clearable::Clearable,
        matchers::{MatchResult, MatchedResult, Matcher},
        Id,
    },
    cli::ui::UiState,
    daemon::{
        channel::RespondedChannel,
        notifications::{DaemonNotification, MatchContext},
    },
};

use super::ansi::{Ansi, AnsiMut};

const NEWLINE_BYTE: u8 = b'\n';

type MatchHandler = dyn FnMut(MatchContext) -> io::Result<()> + Send;
type LineHandler = dyn Fn(&mut Ansi) -> io::Result<()> + Send;

#[derive(Debug, PartialEq)]
pub enum MatcherId {
    Handler(Id),
    Prompt { group: Id, index: usize },
}

struct RegisteredMatcher {
    #[allow(dead_code)]
    id: MatcherId,
    matcher: Matcher,
    mode: MatcherMode,
    on_match: Box<MatchHandler>,
}

struct RegisteredLineProcessor {
    process: Box<LineHandler>,
}

#[derive(Default)]
pub struct TextProcessor {
    matchers: Vec<RegisteredMatcher>,
    processors: Vec<RegisteredLineProcessor>,
    pending_line: AnsiMut,
}

pub enum SystemMessage {
    ConnectionStatus(String),
}

pub trait ProcessorOutputReceiver {
    fn begin_chunk(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn end_chunk(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn new_line(&mut self) -> io::Result<()>;
    fn finish_line(&mut self) -> io::Result<()>;

    /// If we haven't printed a complete line, clear whatever's pending
    fn clear_partial_line(&mut self) -> io::Result<()>;

    fn text(&mut self, text: Ansi) -> io::Result<()>;
    fn system(&mut self, text: SystemMessage) -> io::Result<()>;
    fn notification(&mut self, notification: DaemonNotification) -> io::Result<()>;
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum MatcherMode {
    PartialLine,
    FullLine,
}

enum PerformMatchResult<'a> {
    Matched(&'a mut RegisteredMatcher, MatchedResult),
    Ignored(Ansi),
}

pub trait ProcessorOutputReceiverFactory: Clone + Send {
    type Implementation: ProcessorOutputReceiver + Send;

    fn create(
        &self,
        state: Arc<Mutex<UiState>>,
        connection_id: Id,
        notifier: RespondedChannel,
    ) -> Self::Implementation;
}

impl TextProcessor {
    pub fn process<R: ProcessorOutputReceiver>(
        &mut self,
        text: Ansi,
        receiver: &mut R,
    ) -> io::Result<()> {
        let mut bytes = text.into_inner();

        while bytes.has_remaining() {
            // Read up until a newline from text; push that onto pending_line...
            let (read, has_full_line) =
                if let Some(newline_pos) = bytes.iter().position(|ch| *ch == NEWLINE_BYTE) {
                    let end = newline_pos + 1;
                    self.pending_line.put_slice(&bytes[0..end]);
                    (end, true)
                } else {
                    self.pending_line.put_slice(&bytes);
                    (bytes.len(), false)
                };

            // ... Then process the line
            self.process_pending_line(has_full_line, receiver)?;

            bytes.advance(read);
        }

        Ok(())
    }

    fn process_pending_line<R: ProcessorOutputReceiver>(
        &mut self,
        has_full_line: bool,
        receiver: &mut R,
    ) -> io::Result<()> {
        // Handle trailing carriage returns from previous lines:
        if self.pending_line.starts_with('\r') {
            // This is particularly important for matchers of whole lines, such as prompts
            let mut old_bytes = self.pending_line.take_bytes();
            let trimmed_bytes = old_bytes.split_off(1);
            self.pending_line = AnsiMut::from_bytes(trimmed_bytes);
        }

        if self.pending_line.has_incomplete_code() {
            // If there's some incomplete ANSI code, this becomes a no-op; we'll
            // wait for the next chunk to come in from the network to avoid breaking
            // that up.
            return Ok(());
        }

        receiver.clear_partial_line()?;

        let (match_mode, to_match) = if has_full_line {
            let mut full_line = self.pending_line.take();

            self.perform_processing(&mut full_line)?;

            (MatcherMode::FullLine, full_line)
        } else {
            (MatcherMode::PartialLine, self.pending_line.clone().into())
        };

        let to_print = match self.perform_match(to_match, match_mode) {
            PerformMatchResult::Matched(
                handler,
                MatchedResult {
                    context, remaining, ..
                },
            ) => {
                (handler.on_match)(context)?;
                remaining
            }

            PerformMatchResult::Ignored(text) => Some(text),
        };

        if let Some(to_print) = to_print {
            receiver.text(to_print)?;

            if has_full_line {
                receiver.new_line()?;
            }
        }

        receiver.finish_line()?;

        Ok(())
    }

    pub fn register_matcher<R: 'static + FnMut(MatchContext) -> io::Result<()> + Send>(
        &mut self,
        id: MatcherId,
        matcher: Matcher,
        mode: MatcherMode,
        on_match: R,
    ) {
        self.matchers.push(RegisteredMatcher {
            id,
            matcher,
            mode,
            on_match: Box::new(on_match),
        })
    }

    pub fn register_processor<P: 'static + Fn(&mut Ansi) -> io::Result<()> + Send>(
        &mut self,
        processor: P,
    ) {
        self.processors.push(RegisteredLineProcessor {
            process: Box::new(processor),
        })
    }

    fn perform_processing(&self, line: &mut Ansi) -> io::Result<()> {
        for p in &self.processors {
            (p.process)(line)?;
        }
        Ok(())
    }

    fn perform_match(&mut self, mut to_match: Ansi, mode: MatcherMode) -> PerformMatchResult {
        for m in &mut self.matchers {
            if mode < m.mode {
                continue;
            }
            to_match = match m.matcher.try_match(to_match) {
                MatchResult::Ignored(ansi) => ansi,
                MatchResult::Matched(matched) => {
                    return PerformMatchResult::Matched(m, matched);
                }
            }
        }

        PerformMatchResult::Ignored(to_match)
    }
}

impl Clearable for TextProcessor {
    fn clear(&mut self) {
        self.matchers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TextReceiver {
        outputs: Vec<Ansi>,
    }

    impl ProcessorOutputReceiver for TextReceiver {
        fn clear_partial_line(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn new_line(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn finish_line(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn system(&mut self, _message: SystemMessage) -> io::Result<()> {
            Ok(())
        }

        fn text(&mut self, text: Ansi) -> io::Result<()> {
            self.outputs.push(text);
            Ok(())
        }

        fn notification(&mut self, _notification: DaemonNotification) -> io::Result<()> {
            Ok(())
        }
    }

    fn assert_text_eq(value: &Ansi, expected: &str) {
        assert_eq!(&value[..], expected)
    }

    #[test]
    fn text_processor_full_line() {
        let mut processor = TextProcessor::default();
        let mut receiver = TextReceiver::default();
        processor
            .process("Everything is fine\n".into(), &mut receiver)
            .unwrap();
        assert_text_eq(&receiver.outputs[0], "Everything is fine\n");
    }

    #[test]
    fn text_processor_multi_lines() {
        let mut processor = TextProcessor::default();
        let mut receiver = TextReceiver::default();
        processor
            .process("\nEverything\nIs".into(), &mut receiver)
            .unwrap();
        assert_eq!(receiver.outputs.len(), 3);
        assert_text_eq(&receiver.outputs[0], "\n");
        assert_text_eq(&receiver.outputs[1], "Everything\n");

        // NOTE: The third line of output should have a "save position" control + "Is"
    }

    #[test]
    fn text_processor_carriage_returns() {
        let mut processor = TextProcessor::default();
        let mut receiver = TextReceiver::default();
        processor
            .process("Everything\n\rIs\n".into(), &mut receiver)
            .unwrap();
        assert_eq!(receiver.outputs.len(), 2);
        assert_text_eq(&receiver.outputs[0], "Everything\n");
        assert_text_eq(&receiver.outputs[1], "Is\n");
    }
}
