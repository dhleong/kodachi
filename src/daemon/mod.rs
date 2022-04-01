use std::io::{self, BufRead, Write};

mod channel;
mod commands;
mod protocol;
mod responses;

use commands::DaemonCommand;

use crate::daemon::responses::DaemonResponse;

use self::{
    channel::{Channel, ChannelSource},
    protocol::Request,
};

struct Daemon {}

impl Daemon {
    fn connect(&self, mut channel: Channel, data: commands::Connect) {
        println!("TODO: connect @ {}", data.uri);
        channel.respond(DaemonResponse::Connected { id: 42 })
    }
}

pub async fn daemon<TInput: BufRead, TResponse: 'static + Write>(
    input: TInput,
    response: TResponse,
) -> io::Result<()> {
    let daemon = Daemon {};
    let channels = ChannelSource::new(Box::new(response));

    for read in input.lines() {
        let raw_json = read?;
        let request: Request = serde_json::from_str(&raw_json).unwrap();
        let channel = channels.create_with_request_id(request.id);

        match request.payload {
            DaemonCommand::Quit => break,
            DaemonCommand::Connect(data) => daemon.connect(channel, data),
        }
    }

    Ok(())
}
