use crate::app::{connections::Outgoing, Id, LockableState};

pub async fn handle(mut state: LockableState, connection_id: Id, width: u16, height: u16) {
    let Some(outbox) = state.lock().unwrap().connections.get_outbox(connection_id) else {
        return;
    };

    let _ = outbox.send(Outgoing::WindowSize { width, height }).await;
}
