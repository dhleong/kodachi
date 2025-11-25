use std::{collections::HashMap, io, path::Path};

use async_trait::async_trait;
use bytes::Bytes;
use serde::Serialize;

use crate::{net::Uri, transport::replay::ReplayTransport};

use self::telnet::TelnetTransport;

mod replay;
pub mod telnet;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum TransportEventValue {
    String(String),
    Vec(Vec<TransportEventValue>),
    Map(HashMap<String, TransportEventValue>),
}

#[derive(Clone, Serialize)]
pub struct EventData {
    ns: String,
    name: String,
    payload: Option<TransportEventValue>,
}

pub enum TransportEvent {
    Data(Bytes),
    Event(EventData),
    EndOfPrompt,
    Nop,
}

pub enum TransportNotification {
    WindowSizeUnavailable,
    WindowSize { width: u16, height: u16 },
}

#[async_trait]
pub trait Transport {
    async fn read(&mut self) -> io::Result<TransportEvent>;
    async fn write(&mut self, data: &[u8]) -> io::Result<usize>;
    async fn notify(&mut self, notification: TransportNotification) -> io::Result<()>;
}

pub struct BoxedTransport(Box<dyn Transport + Send>);

impl BoxedTransport {
    pub fn from<T: 'static + Transport + Send>(transport: T) -> BoxedTransport {
        BoxedTransport(Box::new(transport))
    }

    pub async fn replay(path: &Path, buffer_size: usize) -> io::Result<BoxedTransport> {
        let stream = ReplayTransport::for_file(path).await?;
        Ok(BoxedTransport::from(
            TelnetTransport::connect_with_stream(stream, buffer_size).await?,
        ))
    }

    pub async fn connect_uri(uri: Uri, buffer_size: usize) -> io::Result<BoxedTransport> {
        Ok(if uri.tls {
            BoxedTransport::from(
                TelnetTransport::connect_tls(&uri.host, uri.port, buffer_size).await?,
            )
        } else {
            BoxedTransport::from(TelnetTransport::connect(&uri.host, uri.port, buffer_size).await?)
        })
    }
}

#[async_trait]
impl Transport for BoxedTransport {
    async fn read(&mut self) -> io::Result<TransportEvent> {
        (*self.0).read().await
    }

    async fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        (*self.0).write(data).await
    }

    async fn notify(&mut self, notification: TransportNotification) -> io::Result<()> {
        (*self.0).notify(notification).await
    }
}
