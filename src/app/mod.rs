use std::sync::{Arc, LockResult, Mutex, MutexGuard};

use self::connections::Connections;

pub mod clearable;
pub mod completion;
pub mod connections;
pub mod formatters;
pub mod history;
pub mod matchers;
pub mod processing;
pub mod processors;

pub type Id = u64;

#[derive(Default)]
pub struct State {
    pub connections: Connections,
}

#[derive(Default, Clone)]
pub struct LockableState(Arc<Mutex<State>>);

impl LockableState {
    pub fn lock(&mut self) -> LockResult<MutexGuard<State>> {
        self.0.lock()
    }
}
