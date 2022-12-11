use std::{io, net::TcpStream};

use bytes::BytesMut;
use telnet::{self, Telnet};

use super::{Transport, TransportEvent};

pub struct TelnetTransport {
    telnet: Telnet,
}

impl TelnetTransport {
    pub fn connect(host: &str, port: u16, buffer_size: usize) -> io::Result<Self> {
        let tcp = TcpStream::connect((host, port))?;
        Ok(Self {
            telnet: Telnet::from_stream(Box::new(tcp), buffer_size),
        })
    }
}

impl Transport for TelnetTransport {
    fn read(&mut self) -> io::Result<TransportEvent> {
        match self.telnet.read_nonblocking()? {
            telnet::Event::Data(data) => {
                let r: &[u8] = &data;
                let bytes = BytesMut::from(r);
                Ok(TransportEvent::Data(bytes.freeze()))
            }
            telnet::Event::Error(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
            _ => Ok(TransportEvent::Nop),
        }
    }

    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.telnet.write(data)
    }
}
