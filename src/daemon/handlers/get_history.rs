use crate::{
    app::{Id, LockableState},
    daemon::{channel::Channel, protocol::cursors::HistoryCursor, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    default_limit: usize,
    provided_cursor: Option<HistoryCursor>,
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

    let history = connection.sent.lock().unwrap();

    // If the history version doesn't match, throw away the cursor
    let version = history.version();
    let request_cursor = if provided_cursor.as_ref().map(|c| c.version) == Some(version) {
        provided_cursor.clone()
    } else {
        None
    };

    let limit = request_cursor.as_ref().map_or(default_limit, |c| c.limit);
    let offset = request_cursor.as_ref().map_or(0, |c| c.offset);

    let mut entries: Vec<String> = history
        .iter()
        .skip(offset)
        .take(limit + 1)
        .map(|entry| entry.to_owned())
        .collect();

    let cursor = if entries.len() > limit {
        Some(HistoryCursor {
            offset: offset + limit,
            limit,
            version,
            initial_content: None,
        })
    } else {
        None
    };

    if cursor.is_some() {
        entries.pop();
    }

    channel.respond(DaemonResponse::HistoryResult { entries, cursor });
}
