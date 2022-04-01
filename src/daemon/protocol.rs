use serde::{Deserialize, Serialize};

use super::{commands::DaemonCommand, responses::DaemonResponse};

#[derive(Deserialize)]
pub struct Request {
    pub id: u64,

    #[serde(flatten)]
    pub payload: DaemonCommand,
}

#[derive(Serialize)]
pub struct Response {
    pub request_id: u64,

    #[serde(flatten)]
    pub payload: DaemonResponse,
}
