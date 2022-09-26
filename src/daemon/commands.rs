use serde::Deserialize;

use crate::app::{matchers::MatcherSpec, Id};

#[derive(Debug, Deserialize)]
pub struct Connect {
    pub uri: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    Connect(Connect),
    Disconnect {
        connection_id: Id,
    },
    Send {
        connection_id: Id,
        text: String,
    },

    RegisterTrigger {
        connection_id: Id,
        matcher: MatcherSpec,
        handler_id: Id,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientNotification {
    Quit,

    Clear { connection_id: Id },
}
