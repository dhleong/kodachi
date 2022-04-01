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
    mut input: TInput,
    mut response: TResponse,
) -> io::Result<()> {
    let daemon = Daemon {};

    loop {
        let mut read = String::new();
        input.read_line(&mut read)?;

        let request: Request = serde_json::from_str(&read).unwrap();
        let channel = Channel::new(request.id, &mut response);

        match request.payload {
            DaemonCommand::Quit => break,

            DaemonCommand::Connect(data) => daemon.connect(channel, data),
        }
    }

    Ok(())
}
