use std::io;

use crate::{
    app::{Id, LockableState},
    daemon::{channel::Channel, commands},
};

pub async fn handle(
    channel: Channel,
    state: LockableState,
    connection_id: Id,
    params: commands::CompletionParams,
) -> io::Result<()> {
    let words = vec![
        "grayskull".to_string(),
        "magic".to_string(),
        "swift".to_string(),
        "wind".to_string(),
    ];

    channel.respond(crate::daemon::responses::DaemonResponse::CompleteResult { words });
    Ok(())
}
