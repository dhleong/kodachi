use crate::{
    app::{Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    state: LockableState,
    connection_id: Id,
    history_entries: Vec<String>,
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

    let mut history = connection.sent.lock().await;
    history.clear();
    history.insert_many(history_entries);

    // TODO: Would be nice to persist *here*, but for now we just rely
    // on the existing persist-on-disconnect mechanism
}
