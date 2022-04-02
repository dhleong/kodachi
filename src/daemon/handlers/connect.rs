use std::io;

use crate::{
    app::LockableState,
    connection::{self, Uri},
    daemon::{channel::Channel, commands, responses::DaemonResponse},
};

pub async fn handle(
    mut channel: Channel,
    mut state: LockableState,
    data: commands::Connect,
) -> io::Result<()> {
    let id = state.lock().unwrap().connections.allocate_id();
    let uri = Uri::from_string(&data.uri)?;

    channel.respond(DaemonResponse::Connecting { id });

    tokio::spawn(connection::run(uri));

    Ok(())
}
