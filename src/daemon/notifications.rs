use std::{collections::HashMap, ops::Range};

use serde::Serialize;

use crate::app::{processing::ansi::Ansi, Id};

#[derive(Clone, Serialize)]
pub struct MatchedText {
    #[serde(skip_serializing)]
    pub raw: Ansi,
    pub plain: String,
    pub ansi: String,
}

impl MatchedText {
    pub fn from(mut ansi: Ansi) -> Self {
        return MatchedText {
            ansi: (&ansi).to_string(),
            plain: ansi.strip_ansi().to_string(),
            raw: ansi,
        };
    }
}

#[derive(Clone, Serialize)]
pub struct MatchContext {
    pub named: HashMap<String, MatchedText>,
    pub indexed: HashMap<usize, MatchedText>,
    pub full_match_range: Range<usize>,
}

impl MatchContext {
    pub fn take_full_match(&mut self) -> MatchedText {
        self.indexed
            .remove(&0)
            .expect("MatchContext was missing the full_match somehow")
    }
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DaemonNotification {
    Connected,
    Disconnected,
    TriggerMatched {
        handler_id: Id,
        context: MatchContext,
    },
}
