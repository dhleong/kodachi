use crate::{
    app::{persistence::PersistedLines, LockableState, StatePath},
    daemon::channel::Channel,
};

pub async fn handle(
    channel: Channel,
    state: LockableState,
    key: String,
    start_line: usize,
    end_line: usize,
) {
    let output = {
        let shared = {
            let guard = state.lock().unwrap();
            guard.persisted_output.get(&key).cloned()
        };
        if let Some(shared) = shared {
            shared
        } else {
            // One-off test/fetch (?)
            let path = state
                .persisted_state_path(StatePath {
                    key: &key,
                    kind: "output-history",
                })
                .await;
            PersistedLines::load(path).await.unwrap()
        }
    };

    let lines = match output.load_line_range(start_line..=end_line).await {
        Ok(lines) => lines,
        Err(err) => {
            channel.respond(crate::daemon::responses::DaemonResponse::ErrorResult {
                error: err.to_string(),
            });
            return;
        }
    };

    channel.respond(
        crate::daemon::responses::DaemonResponse::PersistedOutputResult {
            start_line,
            end_line,
            lines,
        },
    );
}
