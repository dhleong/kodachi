use serde::Serialize;

use crate::app::Id;

use super::notifications::MatchContext;

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ServerRequest {
    HandleAliasMatch {
        handler_id: Id,
        context: MatchContext,
    },
}
