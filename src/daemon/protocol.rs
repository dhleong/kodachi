use serde::{Deserialize, Serialize};

use crate::app::Id;

use super::{
    commands::{ClientNotification, ClientRequest},
    notifications::DaemonNotification,
    responses::DaemonResponse,
};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Request {
    ForResponse {
        id: u64,

        #[serde(flatten)]
        payload: ClientRequest,
    },

    Notification(ClientNotification),
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

    #[test]
    fn request_deserialization_test() {
        let mut r: Request = serde_json::from_str(r#"{"type":"Quit"}"#).unwrap();
        match r {
            Request::Notification(ClientNotification::Quit) => {}
            _ => assert!(false, "Expected Quit Notification"),
        }

        r = serde_json::from_str(r#"{"id": 9001, "type":"Disconnect", "connection_id": 42}"#)
            .unwrap();
        match r {
            Request::ForResponse {
                id,
                payload: ClientRequest::Disconnect { connection_id },
            } => {
                assert_eq!(id, 9001);
                assert_eq!(connection_id, 42);
            }
            _ => assert!(false, "Expected Disconnect Rquest"),
        }
    }
}
