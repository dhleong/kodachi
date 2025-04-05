use crate::daemon::{channel::RespondedConnectionChannel, handlers::set_prompt_content};

use super::{connections::ConnectionReceiver, LockableState};

pub fn register_processors(
    state: LockableState,
    connection: &mut ConnectionReceiver,
    receiver: RespondedConnectionChannel,
) {
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

    processor.register_auto_prompt_processor(move |line| {
        let mut my_receiver = receiver.clone();
        set_prompt_content::try_handle(
            Some(&mut my_receiver),
            state.clone(),
            connection_id,
            0,
            0,
            line.to_owned(),
            true,
        )?;
        Ok(())
    });
}
