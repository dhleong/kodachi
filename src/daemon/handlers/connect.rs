use std::io::{self, Write};

use telnet::Event;
use tokio::sync::mpsc;

use crate::{
    app::{
        connections::{ConnectionReceiver, Outgoing},
        LockableState,
    },
    daemon::{
        channel::Channel, commands, notifications::DaemonNotification, responses::DaemonResponse,
    },
    net::Uri,
    transport::{telnet::TelnetTransport, Transport},
};

pub fn process_connection<T: Transport, W: Write>(
    mut transport: T,
    mut connection: ConnectionReceiver,
    mut output: W,
) -> io::Result<()> {
    loop {
        match transport.read()? {
            Event::Data(data) => {
                output.write_all(&data)?;
            }
            Event::Error(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            _ => {}
        };

        match connection.outbox.try_recv() {
            Ok(Outgoing::Text(text)) => {
                transport.write(&text.as_bytes())?;
                transport.write(b"\r\n")?;
            }
            Ok(Outgoing::Disconnect) => {
                break;
            }
            Err(mpsc::error::TryRecvError::Empty) => {}
            Err(mpsc::error::TryRecvError::Disconnected) => {
                break;
            }
        }
    }

    Ok(())
}

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    data: commands::Connect,
) -> io::Result<()> {
    let connection = state.lock().unwrap().connections.create();
    let uri = Uri::from_string(&data.uri)?;
    let id = connection.id;

    let mut notifier = channel.respond(DaemonResponse::Connecting { id });

    let transport = TelnetTransport::connect(&uri.host, uri.port, 4096)?;
    let stdout = io::stdout();

    notifier.notify(DaemonNotification::Connected { id });

    process_connection(transport, connection, stdout)?;

    notifier.notify(DaemonNotification::Disconnected { id });

    state.lock().unwrap().connections.drop(id);

    Ok(())
}
