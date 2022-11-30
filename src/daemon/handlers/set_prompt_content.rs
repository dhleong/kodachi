use std::io;

use crate::{
    app::{processing::ansi::Ansi, Id, LockableState},
    cli::ui::prompts::PromptsState,
    daemon::{channel::Channel, responses::DaemonResponse},
};

use super::set_active_prompt_group;

pub async fn handle(
    channel: Channel,
    state: LockableState,
    connection_id: Id,
    group_id: Id,
    prompt_index: usize,
    content: String,
    set_group_active: bool,
) {
    match try_handle(
        state,
        connection_id,
        group_id,
        prompt_index,
        content,
        set_group_active,
    ) {
        Ok(_) => channel.respond(DaemonResponse::OkResult),
        Err(e) => channel.respond(DaemonResponse::ErrorResult {
            error: e.to_string(),
        }),
    };
}

pub fn try_handle(
    mut state: LockableState,
    connection_id: Id,
    group_id: Id,
    prompt_index: usize,
    content: String,
    set_group_active: bool,
) -> io::Result<()> {
    let conn_state =
        if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id) {
            reference.clone()
        } else {
            return Ok(());
        };

    if let Ok(mut ui_state) = conn_state.ui_state.lock() {
        let new_content = Ansi::from(content);
        if ui_state.active_prompt_group == group_id {
            ui_state.prompts.set_index(prompt_index, new_content);
        } else if let Some(group) = ui_state.inactive_prompt_groups.get_mut(group_id) {
            group.set_index(prompt_index, new_content);
        } else {
            let mut group = PromptsState::default();
            group.set_index(prompt_index, new_content);
            ui_state.inactive_prompt_groups.insert(group_id, group);
        }
    }

    if set_group_active {
        set_active_prompt_group::try_handle(state, connection_id, group_id)
    } else {
        Ok(())
    }
}
