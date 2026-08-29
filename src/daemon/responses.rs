use serde::{Deserialize, Serialize};

use crate::app::{persistence::PersistableLine, Id};

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
        nonce: Option<String>,
        persisted_output_key: Option<String>,
        persisted_output_lines: Option<usize>,
    },
    SendResult {
        sent: bool,
    },

    CompleteResult {
        words: Vec<String>,
    },
    PersistedOutputResult {
        start_line: usize,
        end_line: usize,
        lines: Vec<PersistableLine>,
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

#[derive(Clone, Debug, Deserialize)]
pub struct ResponseToServerRequest {
    pub request_id: Id,

    #[serde(flatten)]
    pub payload: ClientResponse,
}
