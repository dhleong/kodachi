use std::collections::HashMap;

use tokio::sync::mpsc;

use super::Id;

#[derive(Clone)]
pub struct Connection {
    pub outbox: mpsc::Sender<String>,
}

pub struct ConnectionReceiver {
    pub id: Id,
    pub outbox: mpsc::Receiver<String>,
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

        let connection = Connection { outbox: outbox_tx };
        self.connections.insert(id, connection);

        ConnectionReceiver {
            id,
            outbox: outbox_rx,
        }
    }

    pub fn drop(&mut self, id: Id) {
        self.connections.remove(&id);
    }

    pub fn get_outbox(&mut self, id: Id) -> Option<mpsc::Sender<String>> {
        if let Some(conn) = self.connections.get(&id) {
            Some(conn.outbox.clone())
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
