use crate::{
    app::{
        matchers::{Matcher, MatcherSpec},
        processing::send::ProcessResult,
        Id, LockableState,
    },
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    matcher: MatcherSpec,
    replacer_spec: String,
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

    let compiled_matcher: Matcher = match matcher.try_into() {
        Ok(compiled) => compiled,
        Err(e) => {
            channel.respond(DaemonResponse::ErrorResult {
                error: format!("{:?}", e).to_string(),
            });
            return;
        }
    };

    if compiled_matcher.options.consume {
        channel.respond(DaemonResponse::ErrorResult {
            error: format!(
                "Invalid matcher ({:?}); must NOT be `consume`",
                compiled_matcher
            ),
        });
        return;
    }

    processor_ref
        .lock()
        .await
        .register_matcher(compiled_matcher, move |context| {
            let replacer = replacer_spec.clone();
            async move {
                // TODO Fill vars in the replacement_spec with those in context
                Ok(ProcessResult::ReplaceWith(replacer))
            }
        });
}
