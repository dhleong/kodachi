use std::io::{self, Write};

use telnet::TelnetEvent;
use tokio::sync::mpsc;

use crate::{app::connections::ConnectionReceiver, transport::telnet::TelnetTransport};

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

pub async fn run(uri: Uri, mut connection: ConnectionReceiver) -> io::Result<()> {
    let mut transport = TelnetTransport::connect(&uri.host, uri.port, 4096)?;
    let mut stdout = io::stdout();

    loop {
        match transport.read()? {
            TelnetEvent::Data(data) => {
                stdout.write_all(&data)?;
            }
            TelnetEvent::Error(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            _ => {}
        };

        match connection.outbox.try_recv() {
            Ok(text) => {
                transport.write(&text.as_bytes())?;
                transport.write(b"\r\n")?;
            }
            Err(mpsc::error::TryRecvError::Empty) => {}
            Err(mpsc::error::TryRecvError::Disconnected) => {
                break;
            }
        }
    }

    Ok(())
}
