use crate::{
    app::{history::HistoryScrollDirection, Id, LockableState},
    daemon::{channel::Channel, protocol::cursors::HistoryCursor, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    direction: HistoryScrollDirection,
    cursor: Option<HistoryCursor>,
) {
    channel.respond(DaemonResponse::ErrorResult {
        error: "Not supported".to_string(),
    });
}
