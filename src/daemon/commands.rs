use serde::Deserialize;

use crate::app::{matchers::MatcherSpec, Id};

#[derive(Debug, Deserialize)]
pub struct Connect {
    pub uri: String,
}

#[derive(Debug, Deserialize)]
pub struct CompletionParams {
    pub word_to_complete: String,
    pub line_to_cursor: String,
    pub line: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    Connect(Connect),
    Disconnect {
        connection_id: Id,
    },
    Send {
        connection_id: Id,
        text: String,
    },

    /// Request suggestions to complete some word in the composer
    CompleteComposer {
        connection_id: Id,
        params: CompletionParams,
    },

    RegisterTrigger {
        connection_id: Id,
        matcher: MatcherSpec,
        handler_id: Id,
    },

    /// This is provided as a convenience for declaring a Prompt line that directly renders
    /// the whole matched line, without modification. For advanced use cases, like extracting
    /// matched groups and rendering those, use [RegisterTrigger] with a consuming Matcher
    /// and [SetPromptContent]
    RegisterPrompt {
        connection_id: Id,
        matcher: MatcherSpec,
        group_id: Id,
        prompt_index: usize,
    },

    // Set the content of a Prompt line. A Prompt line is uniquely identified by the tuple
    // (connection_id, group_id, prompt_index). `group_id` may be any arbitrary unsigned integer;
    // `0` is a good default value. `prompt_index` is similarly any unsigned integer, but clients
    // should prefer sequential numbers starting from `0`.
    // Prompt lines are organized into groups to facilitate multi-line prompts, and switching
    // between prompts based on whichever one is most-recently triggered.
    // If `set_group_active` is true or not provided, the group_id provided here will also be made
    // the active (displayed) prompt group.
    SetPromptContent {
        connection_id: Id,
        group_id: Id,
        prompt_index: usize,
        content: String,
        set_group_active: Option<bool>,
    },

    // May be used to switch the active prompt group without changing any content.
    SetActivePromptGroup {
        connection_id: Id,
        group_id: Id,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientNotification {
    Quit,

    Clear { connection_id: Id },
}
