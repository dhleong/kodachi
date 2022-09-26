use bytes::{Buf, BufMut, BytesMut};
use crossterm::{
    cursor::{RestorePosition, SavePosition},
    terminal::{Clear, ClearType},
};

use crate::{
    app::{
        clearable::Clearable,
        matchers::{MatchResult, Matcher},
        Id,
    },
    daemon::notifications::DaemonNotification,
};

use super::ansi::{Ansi, AnsiMut};

const NEWLINE_BYTE: u8 = b'\n';

struct RegisteredMatcher {
    matcher: Matcher,
    handler: Id,
}

#[derive(Default)]
pub struct TextProcessor {
    matchers: Vec<RegisteredMatcher>,
    pending_line: AnsiMut,
    saving_position: bool,
}

pub enum ProcessorOutput {
    Text(Ansi),
    Notification(DaemonNotification),
}

impl TextProcessor {
    pub fn process(&mut self, text: Ansi, output_chunks: &mut Vec<ProcessorOutput>) {
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
            self.process_pending_line(has_full_line, output_chunks);

            bytes.advance(read);
        }
    }

    fn process_pending_line(
        &mut self,
        has_full_line: bool,
        output_chunks: &mut Vec<ProcessorOutput>,
    ) {
        let result = if !has_full_line {
            // If we *don't* have a full line (and, if we don't already have a SavePosition
            // set, IE from a previous partial line, emit SavePosition) emit the pending
            let mut to_emit = BytesMut::with_capacity(self.pending_line.len() + 8);
            if !self.saving_position {
                self.saving_position = true;
                crate::write_ansi!(to_emit, SavePosition);
            }
            to_emit.put(self.pending_line.take_bytes());
            to_emit.into()
        } else {
            // If we *do* have a full line in pending_line, pop it off and feed it to matchers;
            // if none "consume" the input, emit. If *any* consume, and we have a SavePosition set,
            // emit RestorePosition + Clear
            let to_match = self.pending_line.take();
            let (handler, result) = self.perform_match(to_match);
            match result {
                MatchResult::Ignored(to_emit) => to_emit.into(),
                MatchResult::Consumed { remaining } => {
                    if self.saving_position {
                        self.saving_position = false;

                        let mut to_emit = BytesMut::with_capacity(remaining.len() + 8);
                        crate::write_ansi!(
                            to_emit,
                            RestorePosition,
                            Clear(ClearType::FromCursorDown)
                        );
                        to_emit.put(remaining.into_inner());
                        to_emit.into()
                    } else {
                        remaining
                    }
                }

                MatchResult::Matched { remaining, context } => {
                    if let Some(handler_id) = handler {
                        output_chunks.push(ProcessorOutput::Notification(
                            DaemonNotification::TriggerMatched {
                                handler_id,
                                context,
                            },
                        ))
                    }

                    remaining
                }
            }
        };

        output_chunks.push(ProcessorOutput::Text(result));
    }

    pub fn register(&mut self, handler: Id, matcher: Matcher) {
        self.matchers.push(RegisteredMatcher { matcher, handler })
    }

    fn perform_match(&mut self, mut to_match: Ansi) -> (Option<Id>, MatchResult) {
        for m in &self.matchers {
            to_match = match m.matcher.try_match(to_match) {
                MatchResult::Ignored(ansi) => ansi,
                matched => {
                    // TODO notify about the match
                    return (Some(m.handler), matched);
                }
            }
        }

        return (None, MatchResult::Ignored(to_match));
    }
}

impl Clearable for TextProcessor {
    fn clear(&mut self) {
        self.matchers.clear();
    }
}

#[macro_export]
macro_rules! write_ansi {
    ($bytes:expr $(, $command:expr)* $(,)?) => {{
        let mut writer = $bytes.writer();
        ::crossterm::queue!(writer $(, $command)+)
            .expect("Failed to write ansi");
        $bytes = writer.into_inner();
    }}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_text_eq(value: &ProcessorOutput, expected: &str) {
        match value {
            ProcessorOutput::Text(text) => assert_eq!(&text[..], expected),
            _ => panic!("Expected text output"),
        }
    }

    #[test]
    fn text_processor_full_line() {
        let mut processor = TextProcessor::default();
        let mut outputs = vec![];
        processor.process("Everything is fine\n".into(), &mut outputs);
        assert_text_eq(&outputs[0], "Everything is fine\n");
    }

    #[test]
    fn text_processor_multi_lines() {
        let mut processor = TextProcessor::default();
        let mut outputs = vec![];
        processor.process("\nEverything\nIs".into(), &mut outputs);
        assert_eq!(outputs.len(), 3);
        assert_text_eq(&outputs[0], "\n");
        assert_text_eq(&outputs[1], "Everything\n");

        // NOTE: The third line of output should have a "save position" control + "Is"
    }
}
