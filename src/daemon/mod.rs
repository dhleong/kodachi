use std::io::{self, BufRead, Write};

mod channel;
mod commands;
mod handlers;
mod notifications;
mod protocol;
mod responses;

use commands::DaemonCommand;

use crate::app::LockableState;

use self::{channel::ChannelSource, protocol::Request};

pub async fn daemon<TInput: BufRead, TResponse: 'static + Write + Send>(
    input: TInput,
    response: TResponse,
) -> io::Result<()> {
    let shared_state = LockableState::default();
    let channels = ChannelSource::new(Box::new(response));

    for read in input.lines() {
        let raw_json = read?;
        let request: Request = serde_json::from_str(&raw_json).unwrap();

        let channel = channels.create_with_request_id(request.id);
        let state = shared_state.clone();

        match request.payload {
            DaemonCommand::Quit => break,
            DaemonCommand::Connect(data) => {
                tokio::spawn(handlers::connect::handle(channel, state, data));
            }
            DaemonCommand::Send { connection, text } => {
                tokio::spawn(handlers::send::handle(channel, state, connection, text));
            }
        };
    }

    Ok(())
}
