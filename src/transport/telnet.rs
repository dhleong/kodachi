use std::{io, net::TcpStream};

use telnet::Telnet;

pub struct TelnetTransport {
    telnet: Telnet,
}

impl TelnetTransport {
    pub fn connect(host: &str, port: u16, buffer_size: usize) -> io::Result<Telnet> {
        let tcp = TcpStream::connect((host, port))?;
        Ok(Telnet::from_stream(Box::new(tcp), buffer_size))
    }
}
