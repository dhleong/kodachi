use serde::{Deserialize, Serialize};

use crate::app::Id;

use super::{
    commands::DaemonCommand, notifications::DaemonNotification, responses::DaemonResponse,
};

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

#[derive(Serialize)]
#[serde(untagged)]
pub enum Notification {
    ForConnection {
        connection_id: Id,

        #[serde(flatten)]
        notification: DaemonNotification,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_test() {
        let s = serde_json::to_string(&Notification::ForConnection {
            connection_id: 42,
            notification: DaemonNotification::Connected,
        })
        .unwrap();
        assert_eq!(s, r#"{"connection_id":42,"type":"Connected"}"#);
    }
}
