use std::io::{self, Write};

use telnet::TelnetEvent;

use crate::transport::telnet::TelnetTransport;

pub async fn connection(host: String, port: u16) -> io::Result<()> {
    let mut transport = TelnetTransport::connect(&host, port, 4096)?;
    // let mut stdout = tokio::io::stdout();
    let mut stdout = io::stdout();

    loop {
        match transport.read_nonblocking()? {
            TelnetEvent::Data(data) => {
                // stdout.write_all(&data).await?;
                stdout.write_all(&data)?;
            }
            TelnetEvent::Error(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            _ => {}
        }
    }

    // Ok(())
}
