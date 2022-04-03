use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

use serde::Serialize;

use super::{notifications::DaemonNotification, protocol::Response, responses::DaemonResponse};

#[derive(Clone)]
struct LockedWriter(Arc<Mutex<Box<dyn Write + Send>>>);

impl LockedWriter {
    fn write_json<V: ?Sized + Serialize>(&mut self, value: &V) -> io::Result<()> {
        serde_json::to_writer(&mut self.clone(), &value).expect("Failed to write response");
        self.write_all(b"\n")?;
        self.flush()
    }
}

impl Write for LockedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

pub struct Channel {
    request_id: u64,
    writer: LockedWriter,
}

impl Channel {
    #[allow(dead_code)]
    pub fn notify(&mut self, payload: DaemonNotification) {
        self.writer.write_json(&payload).unwrap();
    }

    pub fn respond(mut self, payload: DaemonResponse) -> RespondedChannel {
        let response = Response {
            request_id: self.request_id,
            payload,
        };

        self.writer.write_json(&response).unwrap();

        RespondedChannel {
            writer: self.writer,
        }
    }
}

pub struct RespondedChannel {
    writer: LockedWriter,
}

impl RespondedChannel {
    pub fn notify(&mut self, payload: DaemonNotification) {
        self.writer.write_json(&payload).unwrap();
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
