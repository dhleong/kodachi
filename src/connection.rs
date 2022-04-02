use std::io::{self, Write};

use telnet::TelnetEvent;

use crate::transport::telnet::TelnetTransport;

pub struct Uri {
    pub host: String,
    pub port: u16,
    pub tls: bool,
}

impl Uri {
    pub fn from_string(uri: &str) -> io::Result<Self> {
        Ok(Self {
            host: uri.to_string(),
            port: 5656,
            tls: false,
        })
    }
}

pub async fn run(uri: Uri) -> io::Result<()> {
    let mut transport = TelnetTransport::connect(&uri.host, uri.port, 4096)?;
    let mut stdout = io::stdout();

    loop {
        match transport.read_nonblocking()? {
            TelnetEvent::Data(data) => {
                stdout.write_all(&data)?;
            }
            TelnetEvent::Error(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            _ => {}
        }
    }

    // Ok(())
}
