use std::{io, path::PathBuf, time::Duration};

use tokio::time::sleep;

use crate::daemon::{
    commands::{ClientRequest, Connect},
    protocol::Request,
    DaemonRequestSource,
};

pub struct DumpReplayRequestSource {
    path: PathBuf,
    state: bool,
}

impl DumpReplayRequestSource {
    pub fn for_path(path: PathBuf) -> Self {
        Self { path, state: false }
    }
}

impl DaemonRequestSource for DumpReplayRequestSource {
    async fn next(&mut self) -> io::Result<Option<Request>> {
        if self.state {
            sleep(Duration::from_secs(5)).await;
            return Ok(None);
        }

        self.state = true;

        Ok(Some(Request::ForResponse {
            id: 0,
            payload: ClientRequest::Connect(Connect {
                uri: "".to_string(),
                config: None,
                replay: Some(self.path.clone()),
            }),
        }))
    }
}
