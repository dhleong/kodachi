use crate::{app::LockableState, daemon::channel::Channel};

pub async fn handle(
    channel: Channel,
    state: LockableState,
    key: String,
    start_line: usize,
    end_line: usize,
) {
    let Some(output) = ({
        let guard = state.lock().unwrap();
        guard.persisted_output.get(&key).cloned()
    }) else {
        return;
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

    channel.respond(crate::daemon::responses::DaemonResponse::PersistedOutputResult { lines });
}
