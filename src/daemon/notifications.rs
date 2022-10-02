use std::{collections::HashMap, ops::Range};

use serde::Serialize;

use crate::app::{processing::ansi::Ansi, Id};

#[derive(Serialize)]
pub struct MatchedText {
    pub plain: String,
    pub ansi: String,
}

impl MatchedText {
    pub fn from(mut ansi: Ansi) -> Self {
        return MatchedText {
            ansi: (&ansi).to_string(),
            plain: ansi.strip_ansi().to_string(),
        };
    }
}

#[derive(Serialize)]
pub struct MatchContext {
    pub named: HashMap<String, MatchedText>,
    pub indexed: HashMap<usize, MatchedText>,
    pub full_match_range: Range<usize>,
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
