use super::{connections::ConnectionReceiver, LockableState};

pub fn register_processors(state: LockableState, connection: &mut ConnectionReceiver) {
    let connection_id = connection.id;
    let mut processor = connection.state.processor.lock().unwrap();

    let completions_state = state.clone();
    processor.register_processor(move |line| {
        if let Some(conn_state) = completions_state
            .clone()
            .lock()
            .unwrap()
            .connections
            .get_state(connection_id)
        {
            conn_state
                .completions
                .lock()
                .unwrap()
                .process_incoming(line);
        }
        Ok(())
    });
}
