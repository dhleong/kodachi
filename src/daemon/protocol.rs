use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub mod cursors;

use crate::app::Id;

use super::{
    commands::{ClientNotification, ClientRequest},
    notifications::DaemonNotification,
    requests::ServerRequest,
    responses::{DaemonResponse, ResponseToServerRequest},
};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Request {
    ForResponse {
        id: Id,

        #[serde(flatten)]
        payload: ClientRequest,
    },

    Response(ResponseToServerRequest),

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

    ServerRequest {
        id: Id,
        connection_id: Id,

        #[serde(flatten)]
        payload: ServerRequest,
    },
}

#[derive(Clone, Default)]
pub struct RequestIdGenerator {
    next_id: Arc<Mutex<Id>>,
}

impl RequestIdGenerator {
    pub async fn next(&mut self) -> Id {
        let mut lock = self.next_id.lock().await;
        let id = *lock;
        let (next_id, _) = id.overflowing_add(1);
        *lock = next_id;
        return id;
    }
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

    #[cfg(test)]
    mod deserialization_tests {
        use crate::{app::formatters::FormatterSpec, daemon::commands::AliasReplacement};

        use super::*;

        #[test]
        fn request_test() {
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

        #[test]
        fn register_alias_test() {
            let r: Request = serde_json::from_str(
                r#"{
                    "id": 9001,
                    "type": "RegisterAlias",
                    "connection_id": 42,
                    "matcher": {
                        "type": "Regex",
                        "source": "^burrito"
                    },
                    "handler_id": 22
                }"#,
            )
            .unwrap();

            let (id, connection_id, _matcher, replacement) = match r {
                Request::ForResponse {
                    id,
                    payload:
                        ClientRequest::RegisterAlias {
                            connection_id,
                            matcher,
                            replacement,
                        },
                } => (id, connection_id, matcher, replacement),
                _ => {
                    assert!(false, "Expected RegisterAlias");
                    panic!();
                }
            };

            assert_eq!(id, 9001);
            assert_eq!(connection_id, 42);
            assert_eq!(replacement, AliasReplacement::Handler { handler_id: 22 });
        }

        #[test]
        fn register_alias_replacement_test() {
            let r: Request = serde_json::from_str(
                r#"{
                    "id": 9001,
                    "type": "RegisterAlias",
                    "connection_id": 42,
                    "matcher": {
                        "type": "Regex",
                        "source": "^burrito"
                    },
                    "replacement_pattern": "make burrito"
                }"#,
            )
            .unwrap();

            let (id, connection_id, _matcher, replacement) = match r {
                Request::ForResponse {
                    id,
                    payload:
                        ClientRequest::RegisterAlias {
                            connection_id,
                            matcher,
                            replacement,
                        },
                } => (id, connection_id, matcher, replacement),
                _ => {
                    assert!(false, "Expected RegisterAlias");
                    panic!();
                }
            };

            assert_eq!(id, 9001);
            assert_eq!(connection_id, 42);
            assert_eq!(
                replacement,
                AliasReplacement::Simple {
                    replacement_pattern: FormatterSpec::Simple("make burrito".to_string())
                }
            );
        }
    }

    mod reqest_id_generator_tests {
        use super::*;

        #[tokio::test]
        async fn request_id_generator_test() {
            let mut generator = RequestIdGenerator::default();
            let zero = generator.next().await;
            let one = generator.next().await;
            assert_eq!(zero, 0);
            assert_eq!(one, 1);
        }

        #[tokio::test]
        async fn cloning_test() {
            let mut generator_one = RequestIdGenerator::default();
            let mut clone = generator_one.clone();
            let zero = clone.next().await;
            let one = generator_one.next().await;
            assert_eq!(zero, 0);
            assert_eq!(one, 1);
        }
    }
}
