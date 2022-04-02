use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use super::{protocol::Response, responses::DaemonResponse};

struct LockedWriter(Arc<Mutex<Box<dyn Write + Send>>>);

impl Write for LockedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

pub struct Channel {
    request_id: u64,
    writer: LockedWriter,
}

impl Channel {
    pub fn respond(&mut self, payload: DaemonResponse) {
        let response = Response {
            request_id: self.request_id,
            payload,
        };

        serde_json::to_writer(&mut self.writer, &response).expect("Failed to write response");
        self.writer.write_all(b"\n").unwrap();
    }
}

pub struct ChannelSource {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl ChannelSource {
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
        }
    }
}

impl ChannelSource {
    pub fn create_with_request_id(&self, request_id: u64) -> Channel {
        Channel {
            request_id,
            writer: LockedWriter(self.writer.clone()),
        }
    }
}
