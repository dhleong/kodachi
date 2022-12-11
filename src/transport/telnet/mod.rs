use std::io;

use async_trait::async_trait;
use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use super::{Transport, TransportEvent};

mod protocol;
mod processor;

use processor::{TelnetProcessor, TelnetEvent};

pub struct TelnetTransport {
    buffer: BytesMut,
    stream: TcpStream,
    telnet: TelnetProcessor,
}

impl TelnetTransport {
    pub async fn connect(host: &str, port: u16, buffer_size: usize) -> io::Result<Self> {
        let buffer = BytesMut::with_capacity(buffer_size);
        let stream = TcpStream::connect((host, port)).await?;
        Ok(Self {
            buffer,
            stream,
            telnet: TelnetProcessor::default(),
        })
    }

    fn pop_transport_event(&mut self) -> io::Result<TransportEvent> {
        match self.telnet.pop() {
            Some(TelnetEvent::Data(bytes)) => Ok(TransportEvent::Data(bytes)),
            None => Ok(TransportEvent::Nop),
        }
    }
}

#[async_trait]
impl Transport for TelnetTransport {
    async fn read(&mut self) -> io::Result<TransportEvent> {
        match self.pop_transport_event()? {
            TransportEvent::Nop => {}, // Nothing to do; proceed with read below
            pending => return Ok(pending), // Return pending events before next read
        }

        self.stream.read_buf(&mut self.buffer).await?;
        self.telnet.enqueue(&mut self.buffer)?;

        self.pop_transport_event()
    }

    async fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.stream.write_all(data).await?;
        Ok(data.len())
    }
}
