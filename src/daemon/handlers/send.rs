use std::io;

use crate::{
    app::{connections::Outgoing, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

async fn process_aliases(
    channel: Channel,
    state: LockableState,
    connection_id: Id,
    text: String,
) -> (Channel, io::Result<Option<String>>) {
    let processor_ref = if let Some(reference) = state
        .clone()
        .lock()
        .unwrap()
        .connections
        .get_send_processor(connection_id)
    {
        reference.clone()
    } else {
        return (channel, Ok(Some(text)));
    };

    let processor = processor_ref.lock().await;
    let result = processor.process(text).await;

    (channel, result)
}

pub async fn handle(channel: Channel, mut state: LockableState, connection_id: Id, text: String) {
    let (channel, text_result) =
        process_aliases(channel, state.clone(), connection_id, text.clone()).await;
    let to_send = match text_result {
        Ok(Some(text)) => text,

        Ok(None) => return, // Input consumed; nothing to send
        Err(err) => {
            channel.respond(DaemonResponse::ErrorResult {
                error: err.to_string(),
            });
            return;
        }
    };

    // Enqueue the processed text to be sent
    let outbox = state.lock().unwrap().connections.get_outbox(connection_id);
    let sent = if let Some(outbox) = outbox {
        outbox.send(Outgoing::Text(to_send.clone())).await.is_ok()
    } else {
        false
    };

    // After sending successfully, process the input text
    if let Some(connection) = state.lock().unwrap().connections.get_state(connection_id) {
        // Add to send history
        let mut sent = connection.sent.lock().unwrap();
        sent.insert(text.clone());

        // Process for completions
        let mut completions = connection.completions.lock().unwrap();
        completions.process_outgoing(text);
    }

    channel.respond(DaemonResponse::SendResult { sent });
}
