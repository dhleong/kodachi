use std::collections::HashMap;

use serde::Serialize;

use crate::app::{processing::ansi::Ansi, Id};

#[derive(Serialize)]
pub struct MatchedText {
    pub plain: String,
    pub ansi: String,
}
impl MatchedText {
    pub fn from_raw_ansi(text: &str) -> Self {
        let as_ansi = Ansi::from(text);
        return MatchedText {
            plain: (&as_ansi).to_string(),
            ansi: std::str::from_utf8(&as_ansi.into_inner())
                .unwrap()
                .to_string(),
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
