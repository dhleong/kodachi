use crate::{
    app::{connections::Outgoing, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

pub async fn handle(channel: Channel, mut state: LockableState, connection_id: Id, text: String) {
    let outbox = state.lock().unwrap().connections.get_outbox(connection_id);
    let sent = if let Some(outbox) = outbox {
        outbox.send(Outgoing::Text(text.clone())).await.is_ok()
    } else {
        false
    };

    if let Some(connection) = state.lock().unwrap().connections.get_state(connection_id) {
        let mut sent = connection.sent.lock().unwrap();
        sent.insert(text.clone());

        let mut completions = connection.completions.lock().unwrap();
        completions.process_outgoing(text);
    }

    channel.respond(DaemonResponse::SendResult { sent });
}
