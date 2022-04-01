use std::io::Write;

use super::{protocol::Response, responses::DaemonResponse};

pub struct Channel<'a, TWrite> {
    request_id: u64,
    write: &'a mut TWrite,
}

impl<'a, TWrite: Write> Channel<'a, TWrite> {
    pub fn new(request_id: u64, write: &'a mut TWrite) -> Self {
        Self { request_id, write }
    }

    pub fn respond(self, payload: DaemonResponse) {
        let response = Response {
            request_id: self.request_id,
            payload,
        };
        serde_json::to_writer(self.write, &response).expect("Failed to write response");
    }
}
