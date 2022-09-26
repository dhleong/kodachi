use std::{
    future::Future,
    io::{self, BufRead, Write},
};

pub mod channel;
mod commands;
mod handlers;
pub mod notifications;
mod protocol;
pub mod responses;

use crate::app::LockableState;

use self::{
    channel::{Channel, ChannelSource},
    commands::{ClientNotification, ClientRequest},
    protocol::Request,
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
    let channels = ChannelSource::new(Box::new(response));

    for read in input.lines() {
        let raw_json = read?;
        let request: Request = serde_json::from_str(&raw_json).unwrap();
        let state = shared_state.clone();

        match request {
            Request::ForResponse {
                id: request_id,
                payload,
            } => {
                let channel = channels.create_with_request_id(request_id);
                dispatch_request(state, channel, payload);
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

        ClientRequest::RegisterTrigger {
            connection_id: connection,
            matcher,
            handler_id,
        } => {
            tokio::spawn(handlers::register_trigger::handle(
                channel, state, connection, matcher, handler_id,
            ));
        }
    }
}
