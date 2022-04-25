use bytes::BytesMut;

use crate::{
    app::{matchers::MatcherSpec, Id},
    daemon::channel::RespondedChannel,
};

struct RegisteredMatcher {
    matcher: MatcherSpec, // TODO compiled Matcher
    handler: Id,
}

#[derive(Default)]
pub struct TextProcessor {
    matchers: Vec<RegisteredMatcher>,
}

impl TextProcessor {
    pub fn process(
        &mut self,
        text: BytesMut,
        connection_id: Id,
        notifier: &mut RespondedChannel,
    ) -> BytesMut {
        // TODO It might be better if we could avoid a dependency on RespondedChannel here,
        // and emit results rather than sending them directly...
        text
    }

    // TODO compiled Matcher
    pub fn register(&mut self, handler: Id, matcher: MatcherSpec) {
        self.matchers.push(RegisteredMatcher { matcher, handler })
    }
}
