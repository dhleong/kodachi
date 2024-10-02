use std::sync::{Arc, Mutex};

use crate::{
    app::{
        processing::text::{ProcessorOutputReceiver, ProcessorOutputReceiverFactory},
        Id,
    },
    daemon::channel::RespondedChannel,
};

use super::UiState;

pub struct ExternalUI {
    state: Arc<Mutex<UiState>>,
    connection_id: Id,
    notifier: RespondedChannel,
}

impl ExternalUI {
    pub fn create(
        state: Arc<Mutex<UiState>>,
        connection_id: Id,
        notifier: RespondedChannel,
    ) -> Self {
        Self {
            state,
            connection_id,
            notifier,
        }
    }
}

impl ProcessorOutputReceiver for ExternalUI {
    fn new_line(&mut self) -> std::io::Result<()> {
        todo!()
    }

    fn finish_line(&mut self) -> std::io::Result<()> {
        todo!()
    }

    fn clear_partial_line(&mut self) -> std::io::Result<()> {
        todo!()
    }

    fn text(&mut self, text: crate::app::processing::ansi::Ansi) -> std::io::Result<()> {
        todo!()
    }

    fn system(&mut self, text: crate::app::processing::text::SystemMessage) -> std::io::Result<()> {
        todo!()
    }

    fn notification(
        &mut self,
        notification: crate::daemon::notifications::DaemonNotification,
    ) -> std::io::Result<()> {
        self.notifier
            .notify(crate::daemon::protocol::Notification::ForConnection {
                connection_id: self.connection_id,
                notification,
            });
        Ok(())
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
