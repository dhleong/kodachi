use crate::{
    app::{matchers::MatcherSpec, processing::text::MatcherId, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    matcher: MatcherSpec,
    handler_id: Id,
) {
    let processor_ref = if let Some(reference) = state
        .lock()
        .unwrap()
        .connections
        .get_processor(connection_id)
    {
        reference.clone()
    } else {
        channel.respond(DaemonResponse::OkResult);
        return;
    };

    let compiled = match matcher.try_into() {
        Ok(compiled) => compiled,
        Err(e) => {
            channel.respond(DaemonResponse::ErrorResult {
                error: format!("{:?}", e).to_string(),
            });
            return;
        }
    };

    // TODO: We can probably refactor this to use channel.for_connection
    // instead of needing BoxedReceiver...
    processor_ref.lock().unwrap().register_matcher(
        MatcherId::Handler(handler_id),
        compiled,
        move |context, mut receiver| {
            receiver.notify(
                crate::daemon::notifications::DaemonNotification::TriggerMatched {
                    handler_id,
                    context,
                },
            )
        },
    );

    channel.respond(DaemonResponse::OkResult);
}
