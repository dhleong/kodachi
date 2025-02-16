use std::io;

use async_trait::async_trait;
use bytes::BytesMut;
use log::trace;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    select,
};
use tokio_native_tls::{TlsConnector, TlsStream};

use self::{
    options::{mccp::CompressableStream, TelnetOptionsManager},
    protocol::TelnetOption,
};

use super::{Transport, TransportEvent, TransportNotification};

mod options;
mod processor;
mod protocol;

use processor::{TelnetEvent, TelnetProcessor};

pub struct TelnetTransport<S: AsyncRead + AsyncWrite> {
    buffer: BytesMut,
    stream: CompressableStream<S>,
    telnet: TelnetProcessor,
    options: TelnetOptionsManager,
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

impl<S: AsyncRead + AsyncWrite + Unpin + Send> TelnetTransport<S> {
    async fn connect_with_stream(mut stream: S, buffer_size: usize) -> io::Result<Self> {
        let buffer = BytesMut::with_capacity(buffer_size);
        let mut options = TelnetOptionsManager::default();
        options.on_connected(&mut stream).await?;

        Ok(Self {
            buffer,
            stream: CompressableStream::new(stream),
            telnet: TelnetProcessor::default(),
            options,
        })
    }

    async fn process_buffer(&mut self) -> io::Result<Option<TransportEvent>> {
        match self.telnet.process_one(&mut self.buffer)? {
            Some(TelnetEvent::Data(bytes)) => Ok(Some(TransportEvent::Data(bytes))),
            Some(TelnetEvent::Negotiate(negotiation, option)) => {
                trace!(target: "telnet", "<< {:?} {:?}", negotiation, option);

                self.options
                    .negotiate(negotiation, option, &mut self.stream)
                    .await?;
                Ok(Some(TransportEvent::Nop))
            }
            Some(TelnetEvent::Subnegotiate(option, data)) => {
                trace!(target: "telnet", "<< SB {:?} {:?} SE", option, data);

                match option {
                    // Start compressing
                    TelnetOption::MCCP2 => {
                        // Anything remaining in our buffer is compressed and needs to be processed
                        // by the stream:
                        self.stream.start_decompressing(Some(&mut self.buffer));
                    }

                    // Otherwise, delgate to the options manager
                    _ => {
                        self.options
                            .subnegotiate(option, data, &mut self.stream)
                            .await?;
                    }
                }

                Ok(Some(TransportEvent::Nop))
            }
            Some(_) => {
                // TODO: Log unexpected event?
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
            match self.process_buffer().await? {
                None => {
                    // Nothing to do (right now); proceed with read below
                    break;
                }
                Some(TransportEvent::Nop) => {
                    // Some unhandled telnet command; loop back and try again
                }
                // Return pending events before next read
                Some(pending) => return Ok(pending),
            }
        }

        select! {
            read = self.stream.read_buf(&mut self.buffer) => {
                if read? == 0 && self.buffer.is_empty() {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }

                self.process_buffer()
                    .await
                    .map(|option| option.unwrap_or(TransportEvent::Nop))
            },

            event = self.options.recv_event() => {
                if let Some(event) = event {
                    Ok(TransportEvent::Event(event))
                } else {
                    Ok(TransportEvent::Nop)
                }
            },
        }
    }

    async fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.stream.write_all(data).await?;
        Ok(data.len())
    }

    async fn notify(&mut self, notification: TransportNotification) -> io::Result<()> {
        self.options.notify(notification, &mut self.stream).await
    }
}
