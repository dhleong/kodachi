use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ExternalUINotification {
    NewLine,
    FinishLine,
    ClearPartialLine,
    Text { ansi: String },
    ConnectionStatus { text: String },
    LocalSend { text: String },
}
