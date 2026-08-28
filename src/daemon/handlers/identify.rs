use crate::{app::LockableState, daemon::commands::Identify};

pub async fn handle(state: LockableState, request: Identify) {
    state.lock().unwrap().app_name = Some(request.app_name);
}
