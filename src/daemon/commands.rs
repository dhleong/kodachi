use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Connect {
    pub uri: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum DaemonCommand {
    Quit,

    Connect(Connect),
}
