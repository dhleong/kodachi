use std::collections::HashMap;

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
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DaemonNotification {
    Connected { id: Id },
    Disconnected { id: Id },
    TriggerMatched { handler: Id, context: MatchContext },
}
