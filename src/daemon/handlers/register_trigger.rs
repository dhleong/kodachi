use crate::{
    app::{matchers::MatcherSpec, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    matcher: MatcherSpec,
    handler_id: Id,
) {
    let connection_ref =
        if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id) {
            reference.clone()
        } else {
            return;
        };
    let mut connection = connection_ref.lock().unwrap();

    let compiled = match matcher.try_into() {
        Ok(compiled) => compiled,
        Err(e) => {
            channel.respond(DaemonResponse::ErrorResult {
                error: format!("{:?}", e).to_string(),
            });
            return;
        }
    };

    connection
        .processor
        .register(handler_id, compiled, move |context, mut receiver| {
            receiver.notify(
                crate::daemon::notifications::DaemonNotification::TriggerMatched {
                    handler_id,
                    context,
                },
            )
        });
}
