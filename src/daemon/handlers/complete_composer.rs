use std::io;

use crate::{
    app::{completion::CompletionParams, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    params: CompletionParams,
) -> io::Result<()> {
    let connection = match state.lock().unwrap().connections.get_state(connection_id) {
        Some(connection) => connection,
        None => {
            channel.respond(DaemonResponse::ErrorResult {
                error: "Not connected".to_string(),
            });
            return Ok(());
        }
    };

    let completions = connection.completions.lock().unwrap();
    let words = completions.suggest(params).take(50).collect();

    channel.respond(DaemonResponse::CompleteResult { words });
    Ok(())
}
