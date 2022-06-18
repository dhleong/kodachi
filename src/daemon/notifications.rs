use serde::Serialize;

use crate::app::Id;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DaemonNotification {
    Connected { id: Id },
    Disconnected { id: Id },
    TriggerMatched { handler: Id },
}
