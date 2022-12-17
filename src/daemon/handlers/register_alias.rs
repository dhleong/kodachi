use crate::{
    app::{matchers::MatcherSpec, processing::send::ProcessResult, Id, LockableState},
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
        .get_send_processor(connection_id)
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

    let receiver = channel.for_connection(connection_id);
    processor_ref
        .lock()
        .await
        .register_matcher(compiled, move |context| {
            let mut receiver = receiver.clone();
            async move {
                // TODO FIXME handle response; timeout; etc
                receiver
                    .request(crate::daemon::requests::ServerRequest::HandleAliasMatch {
                        handler_id,
                        context,
                    })
                    .await;
                Ok(ProcessResult::Stop)
            }
        });

    channel.respond(DaemonResponse::OkResult);
}
