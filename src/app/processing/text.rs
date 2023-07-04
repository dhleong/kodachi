use std::io;

use bytes::Buf;

use crate::{
    app::{
        clearable::Clearable,
        matchers::{MatchResult, Matcher},
        Id,
    },
    daemon::notifications::{DaemonNotification, MatchContext},
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
    printed_index: usize,
    saving_position: bool,
}

pub trait ProcessorOutputReceiver {
    fn begin_chunk(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn end_chunk(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn save_position(&mut self) -> io::Result<()>;
    fn restore_position(&mut self) -> io::Result<()>;
    fn clear_from_cursor_down(&mut self) -> io::Result<()>;
    fn reset_colors(&mut self) -> io::Result<()>;

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
        // Handle trailing carriage returns from previous lines:
        if self.pending_line.chars().next() == Some('\r') {
            // This is particularly important for matchers of whole lines, such as prompts
            let mut old_bytes = self.pending_line.take_bytes();
            let trimmed_bytes = old_bytes.split_off(1);
            self.pending_line = AnsiMut::from_bytes(trimmed_bytes);
        }

        if !has_full_line {
            if self.pending_line.has_incomplete_code() {
                // If there's some incomplete ANSI code, this becomes a no-op; we'll
                // wait for the next chunk to come in from the network to avoid breaking
                // that up.
                return Ok(());
            }

            // If we *don't* have a full line (and, if we don't already have a SavePosition
            // set, IE from a previous partial line, SavePosition first) then emit the pending
            if !self.saving_position {
                self.saving_position = true;
                receiver.save_position()?;
            }

            // Print any un-printed text on this pending line (but keep
            // the text in there for matching whenever we get a full line!)
            let new_end = self.pending_line.len();
            receiver.text((&self.pending_line[self.printed_index..new_end]).into())?;
            self.printed_index = new_end;
        } else {
            // If we *do* have a full line in pending_line, pop it off and feed it to matchers;
            // if none "consume" the input, emit. If *any* consume, and we have a SavePosition set,
            // emit RestorePosition + Clear first
            let mut to_match = self.pending_line.take();
            self.printed_index = 0; // reset

            // Do some passive processing first
            self.perform_processing(&mut to_match)?;

            let (handler, result) = self.perform_match(to_match);
            match result {
                MatchResult::Ignored(to_emit) => receiver.text(to_emit)?,

                MatchResult::Matched {
                    remaining,
                    context,
                    consumed,
                } => {
                    if let Some(handler) = handler {
                        (handler.on_match)(context)?;
                    }

                    if consumed && self.saving_position {
                        self.saving_position = false;
                        receiver.restore_position()?;
                        receiver.clear_from_cursor_down()?;
                    }

                    receiver.text(remaining)?;
                }
            }
        }

        Ok(())
    }

    pub fn register_matcher<R: 'static + FnMut(MatchContext) -> io::Result<()> + Send>(
        &mut self,
        id: MatcherId,
        matcher: Matcher,
        on_match: R,
    ) {
        self.matchers.push(RegisteredMatcher {
            id,
            matcher,
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

    fn perform_match(
        &mut self,
        mut to_match: Ansi,
    ) -> (Option<&mut RegisteredMatcher>, MatchResult) {
        for m in &mut self.matchers {
            to_match = match m.matcher.try_match(to_match) {
                MatchResult::Ignored(ansi) => ansi,
                matched => {
                    return (Some(m), matched);
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

        fn reset_colors(&mut self) -> io::Result<()> {
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
