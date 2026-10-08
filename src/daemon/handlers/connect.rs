use std::{
    future,
    io::{self},
    sync::Mutex,
};

use bytes::Bytes;
use crossterm::{
    event::{Event, EventStream},
    terminal,
};
use futures::{FutureExt as _, StreamExt as _};

use crate::{
    app::{
        connections::{ConnectionReceiver, Outgoing},
        history::History,
        persistence::{receiver::PersistentOutputHandlingReceiver, PersistedLines},
        processing::text::{
            ProcessorOutputReceiver, ProcessorOutputReceiverFactory, SystemMessage, TextProcessor,
            WindowSizeSource,
        },
        processors::register_processors,
        LockableState, StatePath,
    },
    daemon::{
        channel::Channel, commands, notifications::DaemonNotification, responses::DaemonResponse,
    },
    net::Uri,
    transport::{BoxedTransport, Transport, TransportEvent, TransportNotification},
};

use super::configure_connection::apply_config;

pub async fn process_connection<T: Transport, R: ProcessorOutputReceiver>(
    mut transport: T,
    mut connection: ConnectionReceiver,
    receiver: &mut R,
) -> io::Result<()> {
    let mut connected = true;

    // NOTE: It's a bit hacky to do it this way... but it's also
    // much simpler than introducing some kind of boxed type
    // that wraps stream.next().fuse()
    let mut window_size_stream = match receiver.window_size_source() {
        Some(WindowSizeSource::Crossterm) => {
            // Set initial size
            let (width, height) = terminal::size()?;
            transport
                .notify(TransportNotification::WindowSize { width, height })
                .await?;
            Some(EventStream::new())
        }
        Some(WindowSizeSource::External) => None,
        None => {
            // Also, tell the Naws handler we don't support it:
            transport
                .notify(TransportNotification::WindowSizeUnavailable)
                .await?;
            None
        }
    };

    while connected {
        let window_size_event = window_size_stream
            .as_mut()
            .map(|stream| stream.next().boxed().fuse())
            .unwrap_or_else(|| future::pending().boxed().fuse());

        tokio::select! {
            incoming = transport.read() => match incoming? {
                TransportEvent::Data(data) => {
                    let processor = &connection
                        .state
                        .processor;
                    handle_received_text(receiver, processor, data)?;
                },

                TransportEvent::Event(data) => {
                    receiver.notification(DaemonNotification::Event(data))?;
                },

                TransportEvent::EndOfPrompt => {
                    if connection.state.is_auto_prompt_enabled() {
                        let processor = &connection
                            .state
                            .processor;
                        handle_end_of_prompt(receiver, processor)?;
                    }
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
                    Some(Outgoing::WindowSize { width, height }) => {
                        transport.notify(TransportNotification::WindowSize {width, height}).await?;
                    }
                    Some(Outgoing::Disconnect) | None => {
                        connected = false;
                    }
                };
            },

            maybe_event = window_size_event => if let Some(Ok(Event::Resize(width, height))) = maybe_event {
                transport.notify(TransportNotification::WindowSize {width, height}).await?;
            },

        };
    }

    Ok(())
}

pub async fn handle<TUI: ProcessorOutputReceiverFactory>(
    ui: TUI,
    channel: Channel,
    state: LockableState,
    mut data: commands::Connect,
) -> io::Result<()> {
    let mut connection = state.lock().unwrap().connections.create();
    let connection_id = connection.id;

    let mut persisted_output: Option<PersistedLines> = None;
    let mut persisted_input_lines: Option<usize> = None;
    if let Some(config) = data.config.as_ref() {
        apply_config(&mut connection.state, config);

        if let Some(key) = config.persisted_output_key.as_ref() {
            let path = state
                .persisted_state_path(StatePath {
                    key,
                    kind: "output-history",
                })
                .await;
            let lines = PersistedLines::load(path).await?;
            persisted_output = Some(lines.clone());
            state
                .lock()
                .unwrap()
                .persisted_output
                .insert(key.to_string(), lines);
        }

        if let Some(key) = config.persisted_input_key.as_ref() {
            let path = state
                .persisted_state_path(StatePath {
                    key,
                    kind: "input-history",
                })
                .await;
            let history = History::load(&path).await?;
            persisted_input_lines = Some(history.len());
            let mut history_state = connection.state.sent.lock().await;

            // Process loaded history for completions
            let mut completions = connection.state.completions.lock().unwrap();
            for line in history.iter() {
                completions.process_outgoing(line);
            }
            *history_state = history;
        }
    }

    let notifier = channel.respond(DaemonResponse::Connecting {
        connection_id,
        nonce: data.nonce,
        persisted_output_key: data
            .config
            .as_ref()
            .and_then(|config| config.persisted_output_key.clone()),
        persisted_output_lines: persisted_output
            .as_ref()
            .map(|output| output.persisted_len()),
        persisted_input_key: data
            .config
            .as_ref()
            .and_then(|config| config.persisted_input_key.clone()),
        persisted_input_lines,
    });
    let receiver_state = connection.state.ui_state.clone();
    let base_receiver = ui.create(receiver_state.clone(), connection_id, notifier.clone());
    let mut receiver = PersistentOutputHandlingReceiver {
        base: base_receiver,
        persisted_output,
    };
    let processor_receiver = notifier.for_connection(connection_id);

    let transport = if let Some(replay) = data.replay.take() {
        BoxedTransport::replay(replay, 4096).await?
    } else {
        let uri = Uri::from_string(&data.uri)?;
        match BoxedTransport::connect_uri(uri, 4096).await {
            Ok(transport) => transport,
            Err(err) => {
                receiver.begin_chunk()?;
                receiver.system(SystemMessage::ConnectionStatus(format!(
                    "Failed to connect: {err}\n",
                )))?;
                receiver.end_chunk()?;
                receiver.notification(DaemonNotification::Disconnected)?;
                return Ok(());
            }
        }
    };

    register_processors(state.clone(), &mut connection, processor_receiver);

    receiver.notification(DaemonNotification::Connected)?;

    let connection_state = connection.state.clone();
    let result = process_connection(transport, connection, &mut receiver).await;
    if let Err(error) = result {
        match error.kind() {
            io::ErrorKind::UnexpectedEof
            | io::ErrorKind::TimedOut
            | io::ErrorKind::ConnectionReset => {
                let message = if error.kind() == io::ErrorKind::UnexpectedEof {
                    "Disconnected.".to_string()
                } else {
                    format!("Disconnected: {error}")
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
    {
        let mut lock = state.lock().unwrap();
        lock.connections.drop(connection_id);
    }

    if let Some(config) = data.config.as_ref() {
        if let Some(key) = config.persisted_output_key.as_ref() {
            state.lock().unwrap().persisted_output.remove(key);
        }

        // TODO: Would be a bit more robust to persist this regularly,
        // perhaps with a debounce, but this is what Judo did and it's
        // simple for now
        if let Some(key) = config.persisted_input_key.as_ref() {
            let path = state
                .persisted_state_path(StatePath {
                    key,
                    kind: "input-history",
                })
                .await;

            let sent_mutex = connection_state.sent.clone();
            let history = sent_mutex.lock().await;
            history.save_to(&path).await?;
        }
    }

    Ok(())
}

pub fn handle_received_text<R: ProcessorOutputReceiver>(
    receiver: &mut R,
    processor: &Mutex<TextProcessor>,
    text: Bytes,
) -> io::Result<()> {
    receiver.begin_chunk()?;

    processor.lock().unwrap().process(text, receiver)?;

    receiver.end_chunk()
}

pub fn handle_sent_text<R: ProcessorOutputReceiver>(
    receiver: &mut R,
    processor: &Mutex<TextProcessor>,
    text: String,
) -> io::Result<()> {
    receiver.begin_chunk()?;

    processor.lock().unwrap().consume_pending_line()?;
    receiver.system(SystemMessage::LocalSend(text))?;

    receiver.end_chunk()?;

    Ok(())
}

pub fn handle_end_of_prompt<R: ProcessorOutputReceiver>(
    receiver: &mut R,
    processor: &Mutex<TextProcessor>,
) -> io::Result<()> {
    receiver.begin_chunk()?;

    processor.lock().unwrap().on_end_of_prompt(receiver)?;

    receiver.end_chunk()
}
