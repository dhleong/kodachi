use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::mpsc;

use crate::cli::ui::UiState;

use super::{processing::text::TextProcessor, Id};

pub enum Outgoing {
    Text(String),
    Disconnect,
}

#[derive(Default, Clone)]
pub struct ConnectionState {
    pub processor: Arc<Mutex<TextProcessor>>,
    pub ui_state: Arc<Mutex<UiState>>,
}

#[derive(Clone)]
pub struct Connection {
    pub outbox: mpsc::Sender<Outgoing>,
    pub state: ConnectionState,
}

pub struct ConnectionReceiver {
    pub id: Id,
    pub outbox: mpsc::Receiver<Outgoing>,
    pub state: ConnectionState,
}

#[derive(Default)]
pub struct Connections {
    next_id: Id,
    connections: HashMap<Id, Connection>,
}

impl Connections {
    pub fn create(&mut self) -> ConnectionReceiver {
        let id = self.allocate_id();
        let (outbox_tx, outbox_rx) = mpsc::channel(1);

        let state = ConnectionState::default();
        let connection = Connection {
            outbox: outbox_tx,
            state: state.clone(),
        };
        self.connections.insert(id, connection);

        ConnectionReceiver {
            id,
            outbox: outbox_rx,
            state,
        }
    }

    pub fn drop(&mut self, id: Id) {
        self.connections.remove(&id);
    }

    pub fn get_outbox(&mut self, id: Id) -> Option<mpsc::Sender<Outgoing>> {
        if let Some(conn) = self.connections.get(&id) {
            Some(conn.outbox.clone())
        } else {
            None
        }
    }

    pub fn get_state(&mut self, id: Id) -> Option<ConnectionState> {
        if let Some(conn) = self.connections.get(&id) {
            Some(conn.state.clone())
        } else {
            None
        }
    }

    pub fn get_processor(&mut self, id: Id) -> Option<Arc<Mutex<TextProcessor>>> {
        if let Some(conn) = self.connections.get(&id) {
            Some(conn.state.processor.clone())
        } else {
            None
        }
    }

    pub fn get_ui_state(&mut self, id: Id) -> Option<Arc<Mutex<UiState>>> {
        if let Some(conn) = self.connections.get(&id) {
            Some(conn.state.ui_state.clone())
        } else {
            None
        }
    }

    fn allocate_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        return id;
    }
}
