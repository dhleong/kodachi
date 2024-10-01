use crate::{
    app::{
        matchers::{Matcher, MatcherSpec},
        processing::text::{MatcherId, MatcherMode},
        Id, LockableState,
    },
    daemon::{channel::Channel, responses::DaemonResponse},
};

use super::set_prompt_content;

pub fn try_handle(
    mut state: LockableState,
    connection_id: Id,
    matcher: MatcherSpec,
    group_id: Id,
    prompt_index: usize,
) -> DaemonResponse {
    let processor_ref = if let Some(reference) = state
        .lock()
        .unwrap()
        .connections
        .get_processor(connection_id)
    {
        reference.clone()
    } else {
        return DaemonResponse::OkResult;
    };

    let mut compiled: Matcher = match matcher.try_into() {
        Ok(compiled) => compiled,
        Err(e) => {
            return DaemonResponse::ErrorResult {
                error: format!("{:?}", e).to_string(),
            };
        }
    };

    // Prompts should always consume:
    compiled.options.consume = true;

    let id = MatcherId::Prompt {
        group: group_id,
        index: prompt_index,
    };

    processor_ref.lock().unwrap().register_matcher(
        id,
        compiled,
        MatcherMode::PartialLine,
        move |mut context| {
            set_prompt_content::try_handle(
                state.clone(),
                connection_id,
                group_id,
                prompt_index,
                context.take_full_match().ansi,
                true,
            )?;
            Ok(())
        },
    );

    return DaemonResponse::OkResult;
}

pub async fn handle(
    channel: Channel,
    state: LockableState,
    connection_id: Id,
    matcher: MatcherSpec,
    group_id: Id,
    prompt_index: usize,
) {
    channel.respond(try_handle(
        state,
        connection_id,
        matcher,
        group_id,
        prompt_index,
    ));
}
