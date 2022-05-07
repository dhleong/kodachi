use bytes::{BufMut, BytesMut};
use crossterm::{
    cursor::{RestorePosition, SavePosition},
    terminal::{Clear, ClearType},
};

use crate::{
    app::{
        matchers::{MatchResult, Matcher},
        Id,
    },
    daemon::channel::RespondedChannel,
};

use super::ansi::Ansi;

const NEWLINE_BYTE: u8 = b'\n';

struct RegisteredMatcher {
    matcher: Matcher,
    handler: Id,
}

#[derive(Default)]
pub struct TextProcessor {
    matchers: Vec<RegisteredMatcher>,
    pending_line: Ansi,
    saving_position: bool,
}

impl TextProcessor {
    pub fn process(
        &mut self,
        text: BytesMut,
        _connection_id: Id, // TODO
        _notifier: &mut RespondedChannel,
    ) -> BytesMut {
        // Read up until a newline from text; push that onto pending_line
        let has_full_line =
            if let Some(newline_pos) = text.iter().position(|ch| *ch == NEWLINE_BYTE) {
                self.pending_line.put_slice(&text[0..newline_pos]);
                true
            } else {
                self.pending_line.put_slice(&text);
                false
            };

        let result = if !has_full_line {
            // If we *don't* have a full line (and, if we don't already have a SavePosition
            // set, IE from a previous partial line, emit SavePosition) emit the pending
            let mut to_emit = BytesMut::with_capacity(self.pending_line.len() + 8);
            if !self.saving_position {
                self.saving_position = true;
                crate::write_ansi!(to_emit, SavePosition);
            }
            to_emit.put(self.pending_line.take());
            to_emit
        } else {
            // If we *do* have a full line in pending_line, pop it off and feed it to matchers;
            // if none "consume" the input, emit. If *any* consume, and we have a SavePosition set,
            // emit RestorePosition + Clear
            let to_match = self.pending_line.take();

            // TODO It might be better if we could avoid a dependency on RespondedChannel here, and
            // emit results rather than sending them directly...
            match self.perform_match(Ansi::from_bytes(to_match)) {
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
                        remaining.into()
                    }
                }
            }
        };

        return result;
    }

    pub fn register(&mut self, handler: Id, matcher: Matcher) {
        self.matchers.push(RegisteredMatcher { matcher, handler })
    }

    fn perform_match(&mut self, mut to_match: Ansi) -> MatchResult {
        for m in &self.matchers {
            to_match = match m.matcher.try_match(to_match) {
                MatchResult::Ignored(ansi) => ansi,
                consumed => {
                    // TODO notify about the match
                    return consumed;
                }
            }
        }

        return MatchResult::Ignored(to_match);
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
