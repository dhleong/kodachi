use crate::{
    app::{Id, LockableState},
    daemon::{channel::Channel, protocol::cursors::HistoryCursor, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    default_limit: usize,
    request_cursor: Option<HistoryCursor>,
) {
    let connection =
        if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id) {
            reference.clone()
        } else {
            channel.respond(DaemonResponse::ErrorResult {
                error: "Not connected".to_string(),
            });
            return;
        };

    // TODO If the history version doesn't match, throw away the cursor

    let limit = request_cursor.as_ref().map_or(default_limit, |c| c.limit);
    let offset = request_cursor.as_ref().map_or(0, |c| c.offset);

    let mut entries: Vec<String> = connection
        .sent
        .lock()
        .unwrap()
        .iter()
        .skip(offset)
        .take(limit + 1)
        .map(|entry| entry.to_owned())
        .collect();

    let cursor = if entries.len() > limit {
        // TODO encode history "version"
        Some(HistoryCursor {
            offset: offset + limit,
            limit,
            version: 0,
        })
    } else {
        None
    };

    if cursor.is_some() {
        entries.pop();
    }

    channel.respond(DaemonResponse::HistoryResult { entries, cursor });
}
