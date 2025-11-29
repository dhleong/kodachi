use std::path::PathBuf;

use serde::Deserialize;

use crate::app::{
    completion::CompletionParams, formatters::FormatterSpec, history::HistoryScrollDirection,
    matchers::MatcherSpec, Id,
};

use super::protocol::cursors::HistoryCursor;

#[derive(Debug, Deserialize)]
pub struct Connect {
    pub uri: String,

    #[serde(flatten)]
    pub config: Option<ConnectionConfig>,

    pub replay: Option<PathBuf>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum AliasReplacement {
    Handler { handler_id: Id },
    Simple { replacement_pattern: FormatterSpec },
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ConnectionConfig {
    pub auto_prompts: Option<bool>,
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

        /// Unless `false`, `text` will be stored in the "send history" associated
        /// with the connection_id, and can be retrieved later via GetHistory or
        /// ScrollHistory, and may be provided as a suggestion in resposne to CompleteComposer.
        persist: Option<bool>,
    },

    ConfigureConnection {
        connection_id: Id,

        #[serde(flatten)]
        config: ConnectionConfig,
    },

    GetHistory {
        connection_id: Id,
        limit: usize,
        cursor: Option<HistoryCursor>,
    },

    ScrollHistory {
        connection_id: Id,
        direction: HistoryScrollDirection,
        content: String,
        cursor: Option<HistoryCursor>,
    },

    /// Request suggestions to complete some word in the composer
    CompleteComposer {
        connection_id: Id,

        #[serde(flatten)]
        params: CompletionParams,
    },

    RegisterAlias {
        connection_id: Id,
        matcher: MatcherSpec,

        #[serde(flatten)]
        replacement: AliasReplacement,
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

    WindowSize {
        connection_id: Id,
        width: u16,
        height: u16,
    },
    Clear {
        connection_id: Id,
    },
}
