pub mod external_ui;

use std::{collections::HashMap, ops::Range};

use serde::Serialize;

use crate::{
    app::{processing::ansi::Ansi, Id},
    transport::EventData,
};

use self::external_ui::ExternalUINotification;

#[derive(Clone, Debug, Serialize)]
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

#[derive(Clone, Debug, Serialize)]
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
    PromptUpdated {
        group_id: Id,
        index: usize,
        content: MatchedText,
    },
    ActivePromptGroupChanged {
        group_id: Id,
    },
    ExternalUI {
        data: ExternalUINotification,
    },
    Event(EventData),
}
