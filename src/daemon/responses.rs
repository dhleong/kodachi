use serde::Serialize;

use crate::app::Id;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    OkResult,
    ErrorResult { error: String },

    Connecting { connection_id: Id },
    SendResult { sent: bool },

    CompleteResult { words: Vec<String> },
}
