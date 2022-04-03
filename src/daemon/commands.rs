use serde::Deserialize;

use crate::app::Id;

#[derive(Debug, Deserialize)]
pub struct Connect {
    pub uri: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum DaemonCommand {
    Quit,

    Connect(Connect),
    Send { connection: Id, text: String },
}
