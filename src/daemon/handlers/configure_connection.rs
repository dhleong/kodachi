use std::io;

use crate::{
    app::{connections::ConnectionState, Id, LockableState},
    daemon::{channel::Channel, commands::ConnectionConfig, responses::DaemonResponse},
};

pub fn apply_config(connection: &mut ConnectionState, config: &ConnectionConfig) {
    let mut ui_state = connection.ui_state.lock().unwrap();
    if let Some(enable_auto_prompts) = config.auto_prompts {
        ui_state.is_auto_prompt_enabled = enable_auto_prompts;
    }
}

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    connection_id: Id,
    config: ConnectionConfig,
) -> io::Result<()> {
    let mut state = state.lock().unwrap();
    let Some(mut connection) = state.connections.get_state(connection_id) else {
        channel.respond(DaemonResponse::ErrorResult {
            error: format!("Invalid connection ID {connection_id}"),
        });
        return Ok(());
    };

    apply_config(&mut connection, &config);

    channel.respond(DaemonResponse::OkResult);
    Ok(())
}
