use std::{io, path::PathBuf};

use tokio::sync::oneshot;

use crate::daemon::{
    commands::{ClientRequest, Connect},
    protocol::{replay::ReplayConfig, Request},
    DaemonRequestSource,
};

pub struct DumpReplayRequestSource {
    path: PathBuf,
    has_started: bool,
    to_await: Option<oneshot::Receiver<()>>,
}

impl DumpReplayRequestSource {
    pub fn for_path(path: PathBuf) -> Self {
        Self {
            path,
            has_started: false,
            to_await: None,
        }
    }
}

impl DaemonRequestSource for DumpReplayRequestSource {
    async fn next(&mut self) -> io::Result<Option<Request>> {
        if self.has_started {
            if let Some(to_await) = self.to_await.take() {
                let _ = to_await.await;
            }
            return Ok(None);
        }

        let (tx, rx) = oneshot::channel();
        self.has_started = true;
        self.to_await = Some(rx);

        Ok(Some(Request::ForResponse {
            id: 0,
            payload: ClientRequest::Connect(Connect {
                uri: "".to_string(),
                config: None,
                replay: Some(ReplayConfig {
                    path: self.path.clone(),
                    on_complete: tx,
                }),
            }),
        }))
    }
}
