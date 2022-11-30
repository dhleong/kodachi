use crate::app::{clearable::Clearable, Id, LockableState};

pub async fn handle(mut state: LockableState, connection_id: Id) {
    let state = if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id)
    {
        reference
    } else {
        return;
    };
    state.processor.lock().unwrap().clear();
    state.ui_state.lock().unwrap().clear();
}
