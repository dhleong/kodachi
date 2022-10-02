use std::io::{self, Write};

use bytes::BytesMut;
use telnet::Event;
use tokio::sync::mpsc;

use crate::{
    app::{
        connections::{ConnectionReceiver, Outgoing},
        processing::ansi::Ansi,
        LockableState,
    },
    cli::ui::AnsiTerminalWriteUI,
    daemon::{
        channel::{Channel, RespondedChannel},
        commands,
        notifications::DaemonNotification,
        protocol::Notification,
        responses::DaemonResponse,
    },
    net::Uri,
    transport::{telnet::TelnetTransport, Transport},
};

pub fn process_connection<T: Transport, W: Write>(
    mut transport: T,
    mut connection: ConnectionReceiver,
    notifier: RespondedChannel,
    output: W,
) -> io::Result<RespondedChannel> {
    let mut receiver = AnsiTerminalWriteUI {
        connection_id: connection.id,
        notifier,
        output,
    };
    loop {
        match transport.read()? {
            Event::Data(data) => {
                let r: &[u8] = &data;
                let bytes = BytesMut::from(r);
                connection
                    .shared_state
                    .lock()
                    .unwrap()
                    .processor
                    .process(Ansi::from(bytes.freeze()), &mut receiver)?;
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

    Ok(receiver.notifier)
}

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    data: commands::Connect,
) -> io::Result<()> {
    let connection = state.lock().unwrap().connections.create();
    let uri = Uri::from_string(&data.uri)?;
    let connection_id = connection.id;

    let mut notifier = channel.respond(DaemonResponse::Connecting { connection_id });

    let transport = TelnetTransport::connect(&uri.host, uri.port, 4096)?;
    let stdout = io::stdout();

    notifier.notify(Notification::ForConnection {
        connection_id,
        notification: DaemonNotification::Connected,
    });

    notifier = process_connection(transport, connection, notifier, stdout)?;

    notifier.notify(Notification::ForConnection {
        connection_id,
        notification: DaemonNotification::Disconnected,
    });

    state.lock().unwrap().connections.drop(connection_id);

    Ok(())
}
