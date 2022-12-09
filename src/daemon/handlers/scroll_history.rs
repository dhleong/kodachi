use crate::{
    app::{history::HistoryScrollDirection, Id, LockableState},
    daemon::{channel::Channel, protocol::cursors::HistoryCursor, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    state: LockableState,
    connection_id: Id,
    direction: HistoryScrollDirection,
    content: String,
    cursor: Option<HistoryCursor>,
) {
    channel.respond(try_handle(state, connection_id, direction, content, cursor));
}

pub fn try_handle(
    mut state: LockableState,
    connection_id: Id,
    direction: HistoryScrollDirection,
    content: String,
    cursor: Option<HistoryCursor>,
) -> DaemonResponse {
    let connection =
        if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id) {
            reference.clone()
        } else {
            return DaemonResponse::ErrorResult {
                error: "Not connected".to_string(),
            };
        };

    // TODO If the history version doesn't match, throw away the cursor

    let offset = cursor.as_ref().map_or(0, |c| c.offset);
    let version = 0; // TODO

    let history = connection.sent.lock().unwrap();

    let next_offset = match (cursor, direction) {
        (None, HistoryScrollDirection::Older) => history.len().checked_sub(1),
        (Some(_), HistoryScrollDirection::Older) => offset.checked_sub(1),
        (None, HistoryScrollDirection::Newer) => None,
        (Some(_), HistoryScrollDirection::Newer) => Some(offset + 1),
    };
    let next_item = if let Some(next_offset) = next_offset {
        history.iter().nth(next_offset).map(|v| v.to_owned())
    } else {
        None
    };

    if let Some(item) = next_item {
        DaemonResponse::HistoryScrollResult {
            new_content: item,
            cursor: Some(HistoryCursor {
                limit: 1,
                offset: next_offset.unwrap(),
                version,
            }),
        }
    } else {
        DaemonResponse::HistoryScrollResult {
            new_content: content,
            cursor: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_to_most_recent_test() {
        let mut state = LockableState::default();
        let conn = state.lock().unwrap().connections.create();
        let connection_id = conn.id;
        conn.state
            .sent
            .lock()
            .unwrap()
            .insert_many(vec!["First".to_string(), "Second".to_string()]);

        let response = try_handle(
            state,
            connection_id,
            HistoryScrollDirection::Older,
            "".to_string(),
            None,
        );
        let (new_content, _cursor) = match response {
            DaemonResponse::HistoryScrollResult {
                new_content,
                cursor,
            } => (new_content, cursor),
            _ => panic!("Didn't scroll successfully"),
        };

        assert_eq!(new_content, "Second");
    }
}
