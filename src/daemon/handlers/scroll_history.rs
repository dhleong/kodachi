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
            cursor,
            new_content: match direction {
                HistoryScrollDirection::Older => content,

                // TODO Restore initial content
                HistoryScrollDirection::Newer => content,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestContext {
        state: LockableState,
        connection_id: Id,
    }

    impl TestContext {
        fn empty() -> Self {
            let mut state = LockableState::default();
            let conn = state.lock().unwrap().connections.create();
            let connection_id = conn.id;
            TestContext {
                state,
                connection_id,
            }
        }

        fn with_history(entries: Vec<String>) -> Self {
            let mut empty = Self::empty();

            empty
                .state
                .lock()
                .unwrap()
                .connections
                .get_state(empty.connection_id)
                .unwrap()
                .sent
                .lock()
                .unwrap()
                .insert_many(entries);

            return empty;
        }

        fn try_handle(
            &mut self,
            direction: HistoryScrollDirection,
            content: String,
            cursor: Option<HistoryCursor>,
        ) -> DaemonResponse {
            try_handle(
                self.state.clone(),
                self.connection_id,
                direction,
                content,
                cursor,
            )
        }

        fn older(
            &mut self,
            content: String,
            cursor: Option<HistoryCursor>,
        ) -> (String, Option<HistoryCursor>) {
            unpack_response(self.try_handle(HistoryScrollDirection::Older, content, cursor))
        }

        fn newer(
            &mut self,
            content: String,
            cursor: Option<HistoryCursor>,
        ) -> (String, Option<HistoryCursor>) {
            unpack_response(self.try_handle(HistoryScrollDirection::Newer, content, cursor))
        }
    }

    fn unpack_response(response: DaemonResponse) -> (String, Option<HistoryCursor>) {
        match response {
            DaemonResponse::HistoryScrollResult {
                new_content,
                cursor,
            } => (new_content, cursor),
            _ => panic!("Didn't scroll successfully"),
        }
    }

    #[test]
    fn scroll_older_empty_test() {
        let (new_content, cursor) =
            TestContext::empty().older("For the honor of grayskull!".to_string(), None);

        assert_eq!(new_content, "For the honor of grayskull!");
        assert_eq!(cursor, None);
    }

    #[test]
    fn scroll_newer_empty_test() {
        let (new_content, cursor) =
            TestContext::empty().newer("For the honor of grayskull!".to_string(), None);

        assert_eq!(new_content, "For the honor of grayskull!");
        assert_eq!(cursor, None);
    }

    #[test]
    fn scroll_backwards_and_forwards_test() {
        let initial_content = "For the honor of grayskull!";
        let mut context =
            TestContext::with_history(vec!["First".to_string(), "Second".to_string()]);
        let (new_content, cursor1) = context.older(initial_content.to_string(), None);
        assert_eq!(new_content, "Second");

        let (new_content, cursor2) = context.older(new_content.to_string(), cursor1);
        assert_eq!(new_content, "First");

        // We've reached the end
        let (new_content, cursor3) = context.older(new_content.to_string(), cursor2);
        assert_eq!(new_content, "First");
        assert_eq!(cursor3, cursor2);

        let (new_content, _cursor4) = context.newer(new_content.to_string(), cursor3);
        assert_eq!(new_content, "Second");

        // let (new_content, cursor5) = context.newer(new_content, cursor4);
        // assert_eq!(new_content, initial_content);
        // assert_eq!(cursor5, None);
    }
}
