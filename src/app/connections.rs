use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::mpsc;

use super::{processing::text::TextProcessor, Id};

pub enum Outgoing {
    Text(String),
    Disconnect,
}

#[derive(Default)]
pub struct ConnectionState {
    pub processor: TextProcessor,
}

#[derive(Clone)]
pub struct Connection {
    pub outbox: mpsc::Sender<Outgoing>,
    pub shared_state: Arc<Mutex<ConnectionState>>,
}

pub struct ConnectionReceiver {
    pub id: Id,
    pub outbox: mpsc::Receiver<Outgoing>,
    pub shared_state: Arc<Mutex<ConnectionState>>,
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
            shared_state: Arc::new(Mutex::new(state)),
        };
        let state_ref = connection.shared_state.clone();
        self.connections.insert(id, connection);

        ConnectionReceiver {
            id,
            outbox: outbox_rx,
            shared_state: state_ref,
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

    pub fn get_state(&mut self, id: Id) -> Option<Arc<Mutex<ConnectionState>>> {
        if let Some(conn) = self.connections.get(&id) {
            Some(conn.shared_state.clone())
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
