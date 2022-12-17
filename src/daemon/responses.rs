use serde::{Deserialize, Serialize};

use crate::app::Id;

use super::protocol::cursors::HistoryCursor;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    OkResult,
    ErrorResult {
        error: String,
    },

    Connecting {
        connection_id: Id,
    },
    SendResult {
        sent: bool,
    },

    CompleteResult {
        words: Vec<String>,
    },
    HistoryResult {
        entries: Vec<String>,
        cursor: Option<HistoryCursor>,
    },
    HistoryScrollResult {
        new_content: String,
        cursor: Option<HistoryCursor>,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientResponse {
    AliasMatchHandled { replacement: Option<String> },
}

#[derive(Clone, Deserialize)]
pub struct ResponseToServerRequest {
    #[serde(rename = "id")]
    pub request_id: Id,

    #[serde(flatten)]
    pub payload: ClientResponse,
}
