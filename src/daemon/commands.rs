use serde::Deserialize;

use crate::app::{matchers::MatcherSpec, Id};

#[derive(Debug, Deserialize)]
pub struct Connect {
    pub uri: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum DaemonCommand {
    Quit,

    Connect(Connect),
    Disconnect {
        connection: Id,
    },
    Send {
        connection: Id,
        text: String,
    },

    Clear {
        connection: Id,
    },
    RegisterTrigger {
        connection: Id,
        matcher: MatcherSpec,
        handler_id: Id,
    },
}
