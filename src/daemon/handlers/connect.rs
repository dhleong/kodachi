use std::io;

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
    transport::{BoxedTransport, Transport, TransportEvent},
};

pub async fn process_connection<T: Transport, R: ProcessorOutputReceiver>(
    mut transport: T,
    mut connection: ConnectionReceiver,
    receiver: &mut R,
) -> io::Result<()> {
    let mut connected = true;
    while connected {
        tokio::select! {
            incoming = transport.read() => match incoming? {
                TransportEvent::Data(data) => {
                    receiver.begin_chunk()?;

                    connection
                        .state
                        .processor
                        .lock()
                        .unwrap()
                        .process(Ansi::from_bytes(data), receiver)?;

                    receiver.end_chunk()?;
                },

                TransportEvent::Event(data) => {
                    receiver.notification(DaemonNotification::Event(data))?;
                },

                TransportEvent::Nop => {},
            },

            outgoing = connection.outbox.recv() => {
                match outgoing {
                    Some(Outgoing::Text(text)) => {
                        transport.write(&text.as_bytes()).await?;
                        transport.write(b"\r\n").await?;

                        // Also print locally
                        receiver.reset_colors()?;
                        receiver.text(text.into())?;
                        receiver.text("\r\n".into())?;
                    }
                    Some(Outgoing::Disconnect) | None => {
                        connected = false;
                    }
                };
            },
        };
    }

    Ok(())
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
    let receiver_state = connection.state.ui_state.clone();
    let stdout = io::stdout();
    let mut receiver = AnsiTerminalWriteUI::create(receiver_state, connection.id, notifier, stdout);

    let transport = match BoxedTransport::connect_uri(uri, 4096).await {
        Ok(transport) => transport,
        Err(err) => {
            receiver.begin_chunk()?;
            receiver.reset_colors()?;
            receiver.text(format!("Failed to connect: {}\n", err).into())?;
            receiver.end_chunk()?;
            receiver.notification(DaemonNotification::Disconnected)?;
            return Ok(());
        }
    };

    register_processors(state.clone(), &mut connection);

    receiver.notification(DaemonNotification::Connected)?;

    let result = process_connection(transport, connection, &mut receiver).await;
    if let Err(error) = result {
        match error.kind() {
            io::ErrorKind::UnexpectedEof
            | io::ErrorKind::TimedOut
            | io::ErrorKind::ConnectionReset => {
                let message = if error.kind() == io::ErrorKind::UnexpectedEof {
                    "Disconnected.".to_string()
                } else {
                    format!("Disconnected: {}", error)
                };
                receiver.begin_chunk()?;
                receiver.reset_colors()?;
                receiver.text(format!("\n{}\n", message).into())?;
                receiver.end_chunk()?;
            }
            _ => {
                return Err(error);
            }
        }
    }

    receiver.notification(DaemonNotification::Disconnected)?;
    state.lock().unwrap().connections.drop(connection_id);

    Ok(())
}
