use std::io;

use crate::{
    app::{
        matchers::{Matcher, MatcherSpec},
        processing::send::ProcessResult,
        Id, LockableState,
    },
    daemon::{
        channel::Channel,
        requests::ServerRequest,
        responses::{ClientResponse, DaemonResponse},
    },
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

    let compiled: Matcher = match matcher.try_into() {
        Ok(compiled) => compiled,
        Err(e) => {
            channel.respond(DaemonResponse::ErrorResult {
                error: format!("{:?}", e).to_string(),
            });
            return;
        }
    };

    if compiled.options.consume {
        channel.respond(DaemonResponse::ErrorResult {
            error: format!("Invalid matcher ({:?}); must NOT be `consume`", compiled),
        });
        return;
    }

    let receiver = channel.for_connection(connection_id);
    processor_ref
        .lock()
        .await
        .register_matcher(compiled, move |context| {
            let mut receiver = receiver.clone();
            async move {
                let response = receiver
                    .request(ServerRequest::HandleAliasMatch {
                        handler_id,
                        context,
                    })
                    .await?;

                match response {
                    ClientResponse::AliasMatchHandled {
                        replacement: Some(replacement),
                    } => Ok(ProcessResult::ReplaceWith(replacement)),

                    ClientResponse::AliasMatchHandled { replacement: None } => {
                        Ok(ProcessResult::Stop)
                    }

                    #[allow(unreachable_patterns)]
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("Unexpected response: {:?}", response),
                    )),
                }
            }
        });

    channel.respond(DaemonResponse::OkResult);
}
