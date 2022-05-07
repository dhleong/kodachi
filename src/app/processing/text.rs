use bytes::{BufMut, BytesMut};
use crossterm::{
    cursor::{RestorePosition, SavePosition},
    execute,
    terminal::{Clear, ClearType},
};

use crate::{
    app::{
        matchers::{MatchResult, MatcherSpec},
        Id,
    },
    daemon::channel::RespondedChannel,
};

use super::ansi::Ansi;

const NEWLINE_BYTE: u8 = b'\n';

struct RegisteredMatcher {
    matcher: MatcherSpec, // TODO compiled Matcher
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
        // TODO: Read up until a newline from text; push that onto pending_line
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

                let mut writer = to_emit.writer();
                execute!(writer, SavePosition).expect("Failed to write ansi");
                to_emit = writer.into_inner();
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
                        let mut writer = to_emit.writer();
                        execute!(writer, RestorePosition, Clear(ClearType::FromCursorDown))
                            .expect("Failed to write ansi");
                        to_emit = writer.into_inner();
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

    // TODO compiled Matcher
    pub fn register(&mut self, handler: Id, matcher: MatcherSpec) {
        self.matchers.push(RegisteredMatcher { matcher, handler })
    }

    fn perform_match(&mut self, mut to_match: Ansi) -> MatchResult {
        // TODO: Do The Thing.
        MatchResult::Ignored(to_match)
    }
}
