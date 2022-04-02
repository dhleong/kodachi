use crate::{
    app::LockableState,
    daemon::{channel::Channel, commands, responses::DaemonResponse},
};

pub async fn handle(mut channel: Channel, mut state: LockableState, data: commands::Connect) {
    println!("alloc id @ {}", data.uri);
    let id = state.lock().unwrap().connections.allocate_id();
    println!("TODO: connect @ {}", data.uri);
    channel.respond(DaemonResponse::Connected { id })
}
