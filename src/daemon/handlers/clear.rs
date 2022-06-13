use crate::app::{clearable::Clearable, Id, LockableState};

pub async fn handle(mut state: LockableState, connection_id: Id) {
    let connection_ref =
        if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id) {
            reference.clone()
        } else {
            return;
        };
    let mut connection = connection_ref.lock().unwrap();
    connection.processor.clear();
}
