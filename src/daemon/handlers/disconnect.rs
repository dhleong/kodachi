use crate::app::{connections::Outgoing, Id, LockableState};

pub async fn handle(mut state: LockableState, connection_id: Id) {
    let outbox = state.lock().unwrap().connections.get_outbox(connection_id);
    if let Some(outbox) = outbox {
        outbox.send(Outgoing::Disconnect).await.ok();
    }
}
