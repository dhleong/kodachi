use std::io;

use bytes::Buf;

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

pub trait ProcessorOutputReceiver {
    fn save_position(&mut self) -> io::Result<()>;
    fn restore_position(&mut self) -> io::Result<()>;
    fn clear_from_cursor_down(&mut self) -> io::Result<()>;

    fn text(&mut self, text: Ansi) -> io::Result<()>;
    fn notification(&mut self, notification: DaemonNotification) -> io::Result<()>;
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
        if !has_full_line {
            // If we *don't* have a full line (and, if we don't already have a SavePosition
            // set, IE from a previous partial line, SavePosition first) then emit the pending
            if !self.saving_position {
                self.saving_position = true;
                receiver.save_position()?;
            }
            receiver.text(self.pending_line.take_bytes().into())?;
        } else {
            // If we *do* have a full line in pending_line, pop it off and feed it to matchers;
            // if none "consume" the input, emit. If *any* consume, and we have a SavePosition set,
            // emit RestorePosition + Clear first
            let to_match = self.pending_line.take();
            let (handler, result) = self.perform_match(to_match);
            match result {
                MatchResult::Ignored(to_emit) => receiver.text(to_emit)?,
                MatchResult::Consumed { remaining } => {
                    if self.saving_position {
                        self.saving_position = false;

                        receiver.restore_position()?;
                        receiver.clear_from_cursor_down()?;
                    }
                    receiver.text(remaining)?;
                }

                MatchResult::Matched { remaining, context } => {
                    if let Some(handler_id) = handler {
                        receiver.notification(DaemonNotification::TriggerMatched {
                            handler_id,
                            context,
                        })?;
                    }

                    receiver.text(remaining)?;
                }
            }
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TextReceiver {
        outputs: Vec<Ansi>,
    }

    impl ProcessorOutputReceiver for TextReceiver {
        fn save_position(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn restore_position(&mut self) -> io::Result<()> {
            Ok(())
        }

        fn clear_from_cursor_down(&mut self) -> io::Result<()> {
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
}
