use crate::{
    app::{matchers::MatcherSpec, processing::text::MatcherId, Id, LockableState},
    daemon::{channel::Channel, responses::DaemonResponse},
};

use super::set_prompt_content;

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    matcher: MatcherSpec,
    group_id: Id,
    prompt_index: usize,
) {
    let processor_ref = if let Some(reference) = state
        .lock()
        .unwrap()
        .connections
        .get_processor(connection_id)
    {
        reference.clone()
    } else {
        channel.respond(DaemonResponse::OkResult);
        return;
    };

    let compiled = match matcher.try_into() {
        Ok(compiled) => compiled,
        Err(e) => {
            channel.respond(DaemonResponse::ErrorResult {
                error: format!("{:?}", e).to_string(),
            });
            return;
        }
    };

    let id = MatcherId::Prompt {
        group: group_id,
        index: prompt_index,
    };

    processor_ref
        .lock()
        .unwrap()
        .register_matcher(id, compiled, move |mut context, _| {
            set_prompt_content::try_handle(
                state.clone(),
                connection_id,
                group_id,
                prompt_index,
                context.take_full_match().ansi,
                true,
            )?;
            Ok(())
        });

    channel.respond(DaemonResponse::OkResult);
}
