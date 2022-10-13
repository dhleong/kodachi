use std::io;

use crate::{
    app::{Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(channel: Channel, state: LockableState, connection_id: Id, group_id: Id) {
    match try_handle(state, connection_id, group_id) {
        Ok(_) => channel.respond(DaemonResponse::OkResult),
        Err(e) => channel.respond(DaemonResponse::ErrorResult {
            error: e.to_string(),
        }),
    };
}

pub fn try_handle(mut state: LockableState, connection_id: Id, group_id: Id) -> io::Result<()> {
    let conn_state =
        if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id) {
            reference.clone()
        } else {
            return Ok(());
        };

    if let Ok(mut ui_state) = conn_state.ui_state.lock() {
        // TODO set active group
    }

    Ok(())
}
