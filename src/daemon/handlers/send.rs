use crate::{
    app::{connections::Outgoing, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(channel: Channel, mut state: LockableState, connection_id: Id, text: String) {
    let outbox = state.lock().unwrap().connections.get_outbox(connection_id);
    let sent = if let Some(outbox) = outbox {
        outbox.send(Outgoing::Text(text)).await.is_ok()
    } else {
        false
    };

    channel.respond(DaemonResponse::SendResult { sent });
}
