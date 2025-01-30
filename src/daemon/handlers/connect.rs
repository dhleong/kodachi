use std::{
    env,
    fs::File,
    io::{self, Write},
    sync::Mutex,
};

use crate::{
    app::{
        connections::{ConnectionReceiver, Outgoing},
        processing::{
            ansi::Ansi,
            text::{
                ProcessorOutputReceiver, ProcessorOutputReceiverFactory, SystemMessage,
                TextProcessor,
            },
        },
        processors::register_processors,
        LockableState,
    },
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
    let mut dump = if let Ok(filename) = env::var("KODACHI_DUMP") {
        if !filename.is_empty() {
            Some(File::options().append(true).create(true).open(filename)?)
        } else {
            None
        }
    } else {
        None
    };

    let mut connected = true;
    while connected {
        tokio::select! {
            incoming = transport.read() => match incoming? {
                TransportEvent::Data(data) => {
                    if let Some(f) = &mut dump {
                        f.write_all(&data)?;
                    }

                    let processor = &connection
                        .state
                        .processor;
                    handle_received_text(receiver, processor, Ansi::from_bytes(data))?;
                },

                TransportEvent::Event(data) => {
                    receiver.notification(DaemonNotification::Event(data))?;
                },

                TransportEvent::Nop => {},
            },

            outgoing = connection.outbox.recv() => {
                match outgoing {
                    Some(Outgoing::Text(text)) => {
                        transport.write(text.as_bytes()).await?;
                        transport.write(b"\r\n").await?;

                        // Also print locally
                        let processor = &connection
                            .state
                            .processor;
                        handle_sent_text(receiver, processor, text)?;
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

pub async fn handle<TUI: ProcessorOutputReceiverFactory>(
    ui: TUI,
    channel: Channel,
    mut state: LockableState,
    data: commands::Connect,
) -> io::Result<()> {
    let mut connection = state.lock().unwrap().connections.create();
    let uri = Uri::from_string(&data.uri)?;
    let connection_id = connection.id;

    let notifier = channel.respond(DaemonResponse::Connecting { connection_id });
    let receiver_state = connection.state.ui_state.clone();
    let mut receiver = ui.create(receiver_state, connection_id, notifier);

    let transport = match BoxedTransport::connect_uri(uri, 4096).await {
        Ok(transport) => transport,
        Err(err) => {
            receiver.begin_chunk()?;
            receiver.system(SystemMessage::ConnectionStatus(format!(
                "Failed to connect: {}\n",
                err
            )))?;
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
                receiver.system(SystemMessage::ConnectionStatus(message))?;
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

pub fn handle_received_text<R: ProcessorOutputReceiver>(
    receiver: &mut R,
    processor: &Mutex<TextProcessor>,
    text: Ansi,
) -> io::Result<()> {
    receiver.begin_chunk()?;

    processor.lock().unwrap().process(text, receiver)?;

    receiver.end_chunk()
}

pub fn handle_sent_text<R: ProcessorOutputReceiver>(
    receiver: &mut R,
    _processor: &Mutex<TextProcessor>,
    text: String,
) -> io::Result<()> {
    receiver.begin_chunk()?;

    // TODO: Would be nicer to send a SystemMessage, since
    // this might cause Triggers to run unexpectedly
    receiver.system(SystemMessage::LocalSend(text))?;

    // processor
    //     .lock()
    //     .unwrap()
    //     .process(format!("{text}\r\n").into(), receiver)?;

    receiver.end_chunk()?;

    Ok(())
}
