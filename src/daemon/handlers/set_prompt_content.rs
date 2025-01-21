use std::io;

use crate::{
    app::{processing::ansi::Ansi, Id, LockableState},
    daemon::{
        channel::{Channel, ConnectionChannel},
        notifications::{DaemonNotification, MatchedText},
        responses::DaemonResponse,
    },
};

use super::set_active_prompt_group;

pub async fn handle(
    channel: Channel,
    state: LockableState,
    connection_id: Id,
    group_id: Id,
    prompt_index: usize,
    content: Ansi,
    set_group_active: bool,
) {
    match try_handle(
        None,
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
    mut receiver: Option<&mut ConnectionChannel>,
    mut state: LockableState,
    connection_id: Id,
    group_id: Id,
    prompt_index: usize,
    mut content: Ansi,
    set_group_active: bool,
) -> io::Result<()> {
    let conn_state =
        if let Some(reference) = state.lock().unwrap().connections.get_state(connection_id) {
            reference.clone()
        } else {
            return Ok(());
        };

    if let Ok(mut ui_state) = conn_state.ui_state.lock() {
        if ui_state.active_prompt_group == group_id {
            ui_state.prompts.set_index(prompt_index, content.clone());
        } else {
            let group = ui_state.inactive_prompt_groups.entry(group_id).or_default();
            group.set_index(prompt_index, content.clone())
        }

        let plain = content.strip_ansi().to_string();
        if let Some(ref mut receiver) = receiver {
            receiver.notify(DaemonNotification::PromptUpdated {
                group_id,
                index: prompt_index,
                content: MatchedText {
                    plain,
                    ansi: content.to_string(),
                },
            });
        }
    }

    if set_group_active {
        set_active_prompt_group::try_handle(receiver, state, connection_id, group_id)
    } else {
        Ok(())
    }
}
