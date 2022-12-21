use std::{io, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    app::{
        formatters::{Formatter, FormatterSpec},
        matchers::{Matcher, MatcherCompileError, MatcherSpec},
        processing::send::{ProcessResult, SendTextProcessor},
        Id, LockableState,
    },
    daemon::{
        channel::{Channel, ConnectionChannel},
        commands::AliasReplacement,
        requests::ServerRequest,
        responses::{ClientResponse, DaemonResponse},
    },
};

async fn register_handler_matcher(
    channel: ConnectionChannel,
    processor_ref: Arc<Mutex<SendTextProcessor>>,
    matcher: Matcher,
    handler_id: Id,
) {
    processor_ref
        .lock()
        .await
        .register_matcher(matcher, move |context| {
            let mut receiver = channel.clone();
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
}

async fn register_formatter_matcher(
    processor_ref: Arc<Mutex<SendTextProcessor>>,
    matcher: Matcher,
    formatter: FormatterSpec,
) -> Result<(), MatcherCompileError> {
    let formatter: Formatter = formatter.try_into()?;
    processor_ref
        .lock()
        .await
        .register_matcher(matcher, move |context| {
            let replacement = formatter.format(context);
            async move { Ok(ProcessResult::ReplaceWith(replacement)) }
        });

    Ok(())
}

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    matcher: MatcherSpec,
    replacement: AliasReplacement,
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

    match replacement {
        AliasReplacement::Handler { handler_id } => {
            register_handler_matcher(
                channel.for_connection(connection_id),
                processor_ref,
                compiled,
                handler_id,
            )
            .await;
        }

        AliasReplacement::Simple {
            replacement_pattern: formatter,
        } => {
            let result = register_formatter_matcher(processor_ref, compiled, formatter).await;
            if let Err(e) = result {
                channel.respond(DaemonResponse::ErrorResult {
                    error: format!("{:?}", e),
                });
                return;
            }
        }
    };

    channel.respond(DaemonResponse::OkResult);
}
