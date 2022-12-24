use crate::{
    app::{matchers::MatcherSpec, processing::text::MatcherId, Id, LockableState},
    daemon::{channel::Channel, notifications::DaemonNotification, responses::DaemonResponse},
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

    let mut receiver = channel.for_connection(connection_id);
    processor_ref.lock().unwrap().register_matcher(
        MatcherId::Handler(handler_id),
        compiled,
        move |context| {
            receiver.notify(DaemonNotification::TriggerMatched {
                handler_id,
                context,
            });
            Ok(())
        },
    );

    channel.respond(DaemonResponse::OkResult);
}
