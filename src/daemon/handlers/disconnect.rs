use crate::{
    app::{connections::Outgoing, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(channel: Channel, mut state: LockableState, connection_id: Id) {
    let outbox = state.lock().unwrap().connections.get_outbox(connection_id);
    if let Some(outbox) = outbox {
        outbox.send(Outgoing::Disconnect).await.ok();
    }
    channel.respond(DaemonResponse::OkResult);
}
