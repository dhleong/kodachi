use std::io;

use async_trait::async_trait;
use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use super::{Transport, TransportEvent};

mod processor;
mod protocol;

use processor::{TelnetEvent, TelnetProcessor};

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

    fn process_buffer(&mut self) -> io::Result<Option<TransportEvent>> {
        match self.telnet.process_one(&mut self.buffer)? {
            Some(TelnetEvent::Data(bytes)) => Ok(Some(TransportEvent::Data(bytes))),
            Some(_) => {
                // TODO: Log unexpected event
                Ok(Some(TransportEvent::Nop))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl Transport for TelnetTransport {
    async fn read(&mut self) -> io::Result<TransportEvent> {
        loop {
            match self.process_buffer()? {
                None => {
                    // Nothing to do; proceed with read below
                    break;
                }
                Some(TransportEvent::Nop) => {
                    // Some unhandled telnet command; loop back and try again
                }
                // Return pending events before next read
                Some(pending) => return Ok(pending),
            }
        }

        self.stream.read_buf(&mut self.buffer).await?;
        self.process_buffer()
            .map(|option| option.unwrap_or(TransportEvent::Nop))
    }

    async fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.stream.write_all(data).await?;
        Ok(data.len())
    }
}
