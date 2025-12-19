use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ExternalUINotification {
    NewLine,
    FinishLine,
    ClearPartialLine,
    Text {
        ansi: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        plain: Option<String>,
    },
    ConnectionStatus {
        text: String,
    },
    LocalSend {
        text: String,
    },
}
