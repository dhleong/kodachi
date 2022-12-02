use std::{io, thread::sleep, time::Duration};

use bytes::BytesMut;
use telnet::Event;
use tokio::sync::mpsc;

use crate::{
    app::{
        connections::{ConnectionReceiver, Outgoing},
        processing::{ansi::Ansi, text::ProcessorOutputReceiver},
        processors::register_processors,
        LockableState,
    },
    cli::ui::AnsiTerminalWriteUI,
    daemon::{
        channel::Channel, commands, notifications::DaemonNotification, responses::DaemonResponse,
    },
    net::Uri,
    transport::{telnet::TelnetTransport, Transport},
};

const IDLE_SLEEP_DURATION: Duration = Duration::from_millis(12);

pub fn process_connection<T: Transport, R: ProcessorOutputReceiver>(
    mut transport: T,
    mut connection: ConnectionReceiver,
    mut receiver: R,
) -> io::Result<R> {
    loop {
        let mut idle = true;
        match transport.read()? {
            Event::Data(data) => {
                idle = false;

                receiver.begin_chunk()?;
                let r: &[u8] = &data;
                let bytes = BytesMut::from(r);

                connection
                    .state
                    .processor
                    .lock()
                    .unwrap()
                    .process(Ansi::from(bytes.freeze()), &mut receiver)?;

                receiver.end_chunk()?;
            }
            Event::Error(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            _ => {}
        };

        match connection.outbox.try_recv() {
            Ok(Outgoing::Text(text)) => {
                idle = false;
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

        if idle {
            sleep(IDLE_SLEEP_DURATION);
        }
    }

    Ok(receiver)
}

pub async fn handle(
    channel: Channel,
    mut state: LockableState,
    data: commands::Connect,
) -> io::Result<()> {
    let mut connection = state.lock().unwrap().connections.create();
    let uri = Uri::from_string(&data.uri)?;
    let connection_id = connection.id;

    let notifier = channel.respond(DaemonResponse::Connecting { connection_id });

    let transport = TelnetTransport::connect(&uri.host, uri.port, 4096)?;
    let stdout = io::stdout();

    register_processors(state.clone(), &mut connection);

    let receiver_state = connection.state.ui_state.clone();
    let mut receiver = AnsiTerminalWriteUI::create(receiver_state, connection.id, notifier, stdout);
    receiver.notification(DaemonNotification::Connected)?;

    receiver = process_connection(transport, connection, receiver)?;

    receiver.notification(DaemonNotification::Disconnected)?;
    state.lock().unwrap().connections.drop(connection_id);

    Ok(())
}
