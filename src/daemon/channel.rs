use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use serde::Serialize;
use tokio::{sync::broadcast::Sender, time::timeout};

use crate::app::Id;

use super::{
    protocol::{Notification, Response},
    requests::ServerRequest,
    responses::{ClientResponse, DaemonResponse, ResponseToServerRequest},
};

#[derive(Clone)]
struct LockedWriter(Arc<Mutex<Box<dyn Write + Send>>>, Arc<Mutex<()>>);

impl LockedWriter {
    fn write_json<V: ?Sized + Serialize>(&mut self, value: &V) -> io::Result<()> {
        // Lock the shared resource to write the entire value
        let lock_clone = self.clone();
        let _lock = lock_clone.1.lock().unwrap();

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

#[derive(Clone)]
pub struct Channel {
    request_id: u64,
    writer: LockedWriter,
    response_sender: Sender<ResponseToServerRequest>,
}

impl Channel {
    pub fn for_connection(&self, connection_id: Id) -> ConnectionChannel {
        ConnectionChannel {
            connection_id,
            writer: self.writer.clone(),
            response_sender: self.response_sender.clone(),
        }
    }

    pub fn notify(&mut self, payload: Notification) {
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
    pub fn notify(&mut self, payload: Notification) {
        self.writer.write_json(&payload).unwrap();
    }
}

#[derive(Clone)]
pub struct ConnectionChannel {
    connection_id: Id,
    writer: LockedWriter,
    response_sender: Sender<ResponseToServerRequest>,
}

impl ConnectionChannel {
    pub async fn request(&mut self, payload: ServerRequest) -> io::Result<ClientResponse> {
        let id = 0; // FIXME
        self.writer
            .write_json(&Notification::ServerRequest {
                id,
                connection_id: self.connection_id,
                payload,
            })
            .unwrap();

        let mut receiver = self.response_sender.subscribe();
        loop {
            match timeout(Duration::from_millis(100), receiver.recv()).await {
                Ok(Ok(response)) => {
                    if response.request_id == id {
                        return Ok(response.payload);
                    }
                }
                Ok(err) => {
                    // Probably a RecvError meaning the sender is gone. This.. shouldn't happen
                    return Err(io::ErrorKind::TimedOut.into());
                }
                Err(_) => {
                    return Err(io::ErrorKind::TimedOut.into());
                }
            }
        }
    }
}

pub struct ChannelSource {
    response_sender: Sender<ResponseToServerRequest>,
    writer: LockedWriter,
}

impl ChannelSource {
    pub fn new(
        writer: Box<dyn Write + Send>,
        response_sender: Sender<ResponseToServerRequest>,
    ) -> Self {
        Self {
            writer: LockedWriter(Arc::new(Mutex::new(writer)), Arc::new(Mutex::new(()))),
            response_sender,
        }
    }
}

impl ChannelSource {
    pub fn create_with_request_id(&self, request_id: u64) -> Channel {
        Channel {
            request_id,
            writer: self.writer.clone(),
            response_sender: self.response_sender.clone(),
        }
    }
}
