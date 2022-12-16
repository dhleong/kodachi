use crate::{
    app::{connections::Outgoing, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

async fn process_aliases(
    channel: Channel,
    state: LockableState,
    connection_id: Id,
    text: String,
) -> (Channel, String) {
    let processor_ref = if let Some(reference) = state
        .clone()
        .lock()
        .unwrap()
        .connections
        .get_send_processor(connection_id)
    {
        reference.clone()
    } else {
        return (channel, text);
    };

    (channel, text)
}

pub async fn handle(channel: Channel, mut state: LockableState, connection_id: Id, text: String) {
    let (channel, text) = process_aliases(channel, state.clone(), connection_id, text).await;

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
