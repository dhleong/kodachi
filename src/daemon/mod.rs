use std::{
    future::Future,
    io::{self, BufRead, Write},
};

pub mod channel;
mod commands;
mod handlers;
pub mod notifications;
pub mod protocol;
pub mod requests;
pub mod responses;

use crate::app::LockableState;

use self::{
    channel::{Channel, ChannelSource},
    commands::{ClientNotification, ClientRequest},
    protocol::{Request, RequestIdGenerator},
};

fn launch<T>(handler: T)
where
    T: Future<Output = io::Result<()>> + Send + 'static,
{
    tokio::spawn(async {
        if let Err(e) = handler.await {
            panic!("ERR: {}", e);
        }
    });
}

pub async fn daemon<TInput: BufRead, TResponse: 'static + Write + Send>(
    input: TInput,
    response: TResponse,
) -> io::Result<()> {
    let shared_state = LockableState::default();
    let (to_listeners, _) = tokio::sync::broadcast::channel(1);
    let request_ids = RequestIdGenerator::default();
    let channels = ChannelSource::new(Box::new(response), request_ids, to_listeners.clone());

    for read in input.lines() {
        let raw_json = read?;
        let request: Request = match serde_json::from_str(&raw_json) {
            Ok(request) => request,
            Err(err) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unable to parse input `{}`: {}", raw_json, err),
                ));
            }
        };
        let state = shared_state.clone();

        match request {
            Request::ForResponse {
                id: request_id,
                payload,
            } => {
                let channel = channels.create_with_request_id(request_id);
                dispatch_request(state, channel, payload);
            }

            Request::Response(response) => {
                // NOTE: We ignore errors here; there may be no listeners, and that's okay
                to_listeners.send(response).ok();
            }

            Request::Notification(ClientNotification::Clear { connection_id }) => {
                tokio::spawn(handlers::clear::handle(state, connection_id));
            }

            Request::Notification(ClientNotification::Quit) => break,
        };
    }

    Ok(())
}

fn dispatch_request(state: LockableState, channel: Channel, payload: ClientRequest) {
    match payload {
        ClientRequest::Connect(data) => {
            launch(handlers::connect::handle(channel, state, data));
        }
        ClientRequest::Disconnect {
            connection_id: connection,
        } => {
            tokio::spawn(handlers::disconnect::handle(state, connection));
        }
        ClientRequest::Send {
            connection_id: connection,
            text,
        } => {
            tokio::spawn(handlers::send::handle(channel, state, connection, text));
        }

        ClientRequest::CompleteComposer {
            connection_id: connection,
            params,
        } => {
            tokio::spawn(handlers::complete_composer::handle(
                channel, state, connection, params,
            ));
        }

        ClientRequest::GetHistory {
            connection_id,
            limit,
            cursor,
        } => {
            tokio::spawn(handlers::get_history::handle(
                channel,
                state,
                connection_id,
                limit,
                cursor,
            ));
        }

        ClientRequest::RegisterAlias {
            connection_id,
            matcher,
            handler_id,
        } => {
            tokio::spawn(handlers::register_alias::handle(
                channel,
                state,
                connection_id,
                matcher,
                handler_id,
            ));
        }

        ClientRequest::RegisterPrompt {
            connection_id: connection,
            matcher,
            group_id,
            prompt_index,
        } => {
            tokio::spawn(handlers::register_prompt::handle(
                channel,
                state,
                connection,
                matcher,
                group_id,
                prompt_index,
            ));
        }

        ClientRequest::RegisterTrigger {
            connection_id: connection,
            matcher,
            handler_id,
        } => {
            tokio::spawn(handlers::register_trigger::handle(
                channel, state, connection, matcher, handler_id,
            ));
        }

        ClientRequest::ScrollHistory {
            connection_id,
            direction,
            content,
            cursor,
        } => {
            tokio::spawn(handlers::scroll_history::handle(
                channel,
                state,
                connection_id,
                direction,
                content,
                cursor,
            ));
        }

        ClientRequest::SetPromptContent {
            connection_id,
            group_id,
            prompt_index,
            content,
            set_group_active,
        } => {
            tokio::spawn(handlers::set_prompt_content::handle(
                channel,
                state,
                connection_id,
                group_id,
                prompt_index,
                content,
                set_group_active.unwrap_or(true),
            ));
        }

        ClientRequest::SetActivePromptGroup {
            connection_id,
            group_id,
        } => {
            tokio::spawn(handlers::set_active_prompt_group::handle(
                channel,
                state,
                connection_id,
                group_id,
            ));
        }
    }
}
