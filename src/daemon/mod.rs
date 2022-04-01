use std::io::{self, BufRead, Write};

mod channel;
mod commands;
mod protocol;
mod responses;

use commands::DaemonCommand;

use crate::daemon::responses::DaemonResponse;

use self::{channel::Channel, protocol::Request};

struct Daemon {}

impl Daemon {
    fn connect<'a, W: Write>(&self, channel: Channel<'a, W>, data: commands::Connect) {
        println!("TODO: connect @ {}", data.uri);
        channel.respond(DaemonResponse::Connected { id: 42 })
    }
}

pub async fn daemon<TInput: BufRead, TResponse: Write>(
    input: TInput,
    mut response: TResponse,
) -> io::Result<()> {
    let daemon = Daemon {};

    for read in input.lines() {
        let raw_json = read?;
        let request: Request = serde_json::from_str(&raw_json).unwrap();
        let channel = Channel::new(request.id, &mut response);

        match request.payload {
            DaemonCommand::Quit => break,

            DaemonCommand::Connect(data) => daemon.connect(channel, data),
        }
    }

    Ok(())
}
