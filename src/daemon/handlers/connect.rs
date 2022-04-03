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
    let connection = state.lock().unwrap().connections.create();
    let uri = Uri::from_string(&data.uri)?;
    let id = connection.id;

    channel.respond(DaemonResponse::Connecting { id });

    connection::run(uri, connection).await?;

    state.lock().unwrap().connections.drop(id);

    Ok(())
}
