use std::io;

use async_trait::async_trait;
use bytes::BytesMut;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use tokio_native_tls::{TlsConnector, TlsStream};

use super::{Transport, TransportEvent};

mod processor;
mod protocol;

use processor::{TelnetEvent, TelnetProcessor};

pub struct TelnetTransport<S: AsyncRead + AsyncWrite> {
    buffer: BytesMut,
    stream: S,
    telnet: TelnetProcessor,
}

impl TelnetTransport<TcpStream> {
    pub async fn connect(host: &str, port: u16, buffer_size: usize) -> io::Result<Self> {
        Self::connect_with_stream(TcpStream::connect((host, port)).await?, buffer_size).await
    }
}

impl TelnetTransport<TlsStream<TcpStream>> {
    pub async fn connect_tls(host: &str, port: u16, buffer_size: usize) -> io::Result<Self> {
        let tcp = TcpStream::connect((host, port)).await?;
        let connector = match native_tls::TlsConnector::builder().build() {
            Ok(connector) => connector,
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
        };
        let cx = TlsConnector::from(connector);

        let stream = match cx.connect(host, tcp).await {
            Ok(stream) => stream,
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err)),
        };

        Self::connect_with_stream(stream, buffer_size).await
    }
}

impl<S: AsyncRead + AsyncWrite> TelnetTransport<S> {
    async fn connect_with_stream(stream: S, buffer_size: usize) -> io::Result<Self> {
        let buffer = BytesMut::with_capacity(buffer_size);
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
impl<S: AsyncRead + AsyncWrite + Unpin + Send> Transport for TelnetTransport<S> {
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
