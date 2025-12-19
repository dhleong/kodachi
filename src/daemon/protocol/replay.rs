use std::path::PathBuf;

use tokio::sync::oneshot;

#[derive(Debug)]
pub struct ReplayConfig {
    pub path: PathBuf,
    pub on_complete: oneshot::Sender<()>,
}
