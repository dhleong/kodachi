use crate::daemon::{channel::Channel, commands, responses::DaemonResponse};

pub async fn handle(mut channel: Channel, data: commands::Connect) {
    println!("TODO: connect @ {}", data.uri);
    channel.respond(DaemonResponse::Connected { id: 42 })
}
