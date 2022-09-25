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
        connection_id: Id,
    },
    Send {
        connection_id: Id,
        text: String,
    },

    Clear {
        connection: Id,
    },
    RegisterTrigger {
        connection_id: Id,
        matcher: MatcherSpec,
        handler_id: Id,
    },
}
