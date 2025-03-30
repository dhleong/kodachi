use std::io;

use crate::{
    app::{Id, LockableState},
    daemon::{
        channel::{Channel, ConnectionChannel, ConnectionNotifier},
        notifications::DaemonNotification,
        responses::DaemonResponse,
    },
};

pub async fn handle(channel: Channel, state: LockableState, connection_id: Id, group_id: Id) {
    match try_handle::<ConnectionChannel>(None, state, connection_id, group_id) {
        Ok(_) => channel.respond(DaemonResponse::OkResult),
        Err(e) => channel.respond(DaemonResponse::ErrorResult {
            error: e.to_string(),
        }),
    };
}

pub fn try_handle<N: ConnectionNotifier>(
    receiver: Option<&mut N>,
    mut state: LockableState,
    connection_id: Id,
    group_id: Id,
) -> io::Result<()> {
    let conn_state =
        if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id) {
            reference.clone()
        } else {
            return Ok(());
        };

    if let Ok(mut ui_state) = conn_state.ui_state.lock() {
        let previous_group_id = ui_state.active_prompt_group;
        if previous_group_id != group_id {
            // Retrieve the requested group from the inactive list, if there is one,
            // or create an empty group record otherwise.
            let mut group = ui_state
                .inactive_prompt_groups
                .remove(group_id)
                .unwrap_or_default();

            // Swap the currently-active state into group, and vice versa
            std::mem::swap(&mut ui_state.prompts, &mut group);

            // Stash group (now referencing the previously-active group) into the inactive list
            ui_state
                .inactive_prompt_groups
                .insert(previous_group_id, group);

            // And finally, update the active ID
            ui_state.active_prompt_group = group_id;

            if let Some(receiver) = receiver {
                receiver.notify(DaemonNotification::ActivePromptGroupChanged { group_id });
            }
        }
    }

    Ok(())
}
