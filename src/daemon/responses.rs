use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum DaemonResponse {
    Connected { id: u64 },
}
