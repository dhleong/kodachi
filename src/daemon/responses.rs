use serde::Serialize;

use crate::app::Id;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    Connecting { id: Id },
}
