use bytes::BytesMut;
use regex::Regex;

use crate::{
    app::{matchers::MatcherSpec, Id},
    daemon::channel::RespondedChannel,
};

use super::ansi::Ansi;

struct RegisteredMatcher {
    matcher: MatcherSpec, // TODO compiled Matcher
    handler: Id,
}

#[derive(Default)]
pub struct TextProcessor {
    matchers: Vec<RegisteredMatcher>,
    pending_line: Ansi,
}

impl TextProcessor {
    pub fn process(
        &mut self,
        text: BytesMut,
        connection_id: Id,
        notifier: &mut RespondedChannel,
    ) -> BytesMut {
        // TODO: Read up until a newline from text; push that onto pending_line
        // TODO: If we *don't* have a full line (and, if we don't already have a SavePosition set, IE from
        // a previous partial line, emit SavePosition) emit the pending
        // TODO: If we *do* have a full line in pending_Line, pop it off and feed it to matchers;
        // if none "consume" the input, emit. If *any* consume, and we have a SavePosition set,
        // emit RestorePosition + Clear(ClearType::UntilNewLine)
        self.pending_line.put_slice(&text);
        Regex::new("foo")
            .unwrap()
            .find(&self.pending_line)
            .expect("hi");

        // TODO It might be better if we could avoid a dependency on RespondedChannel here,
        // and emit results rather than sending them directly...
        self.pending_line.take()
    }

    // TODO compiled Matcher
    pub fn register(&mut self, handler: Id, matcher: MatcherSpec) {
        self.matchers.push(RegisteredMatcher { matcher, handler })
    }
}
