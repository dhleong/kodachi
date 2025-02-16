use std::{
    io,
    sync::{Arc, Mutex},
};

use crate::{
    app::{
        processing::{
            ansi::Ansi,
            text::{
                ProcessorOutputReceiver, ProcessorOutputReceiverFactory, SystemMessage,
                WindowSizeSource,
            },
        },
        Id,
    },
    daemon::{
        channel::RespondedChannel,
        notifications::{external_ui::ExternalUINotification, DaemonNotification},
        protocol::Notification,
    },
};

use super::UiState;

pub struct ExternalUI {
    connection_id: Id,
    notifier: RespondedChannel,
}

impl ExternalUI {
    pub fn create(
        _state: Arc<Mutex<UiState>>,
        connection_id: Id,
        notifier: RespondedChannel,
    ) -> Self {
        Self {
            connection_id,
            notifier,
        }
    }
}

impl ProcessorOutputReceiver for ExternalUI {
    fn window_size_source(&self) -> Option<WindowSizeSource> {
        Some(WindowSizeSource::External)
    }

    fn new_line(&mut self) -> std::io::Result<()> {
        self.send_external_ui(ExternalUINotification::NewLine)
    }

    fn finish_line(&mut self) -> std::io::Result<()> {
        self.send_external_ui(ExternalUINotification::FinishLine)
    }

    fn clear_partial_line(&mut self) -> std::io::Result<()> {
        self.send_external_ui(ExternalUINotification::ClearPartialLine)
    }

    fn text(&mut self, text: Ansi) -> std::io::Result<()> {
        self.send_external_ui(ExternalUINotification::Text {
            ansi: text.to_string(),
        })
    }

    fn system(&mut self, text: SystemMessage) -> std::io::Result<()> {
        match text {
            SystemMessage::ConnectionStatus(status) => {
                self.send_external_ui(ExternalUINotification::ConnectionStatus { text: status })
            }

            SystemMessage::LocalSend(sent) => {
                self.send_external_ui(ExternalUINotification::LocalSend { text: sent })
            }
        }
    }

    fn notification(&mut self, notification: DaemonNotification) -> std::io::Result<()> {
        self.notifier.notify(Notification::ForConnection {
            connection_id: self.connection_id,
            notification,
        });
        Ok(())
    }
}

impl ExternalUI {
    fn send_external_ui(&mut self, data: ExternalUINotification) -> io::Result<()> {
        self.notification(DaemonNotification::ExternalUI { data })
    }
}

#[derive(Clone, Copy)]
pub struct ExternalUIFactory;

impl ProcessorOutputReceiverFactory for ExternalUIFactory {
    type Implementation = ExternalUI;

    fn create(
        &self,
        state: Arc<Mutex<UiState>>,
        connection_id: Id,
        notifier: RespondedChannel,
    ) -> Self::Implementation {
        ExternalUI::create(state, connection_id, notifier)
    }
}
