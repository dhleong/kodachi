use std::{io, net::TcpStream};

use telnet::{Telnet, TelnetEvent};

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

    pub fn read(&mut self) -> io::Result<TelnetEvent> {
        self.telnet.read_nonblocking()
    }

    pub fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.telnet.write(data)
    }
}
