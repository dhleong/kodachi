use serde::Serialize;

use crate::app::Id;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    ErrorResult { error: String },

    Connecting { id: Id },
    SendResult { sent: bool },
}
