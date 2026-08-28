use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, LockResult, Mutex, MutexGuard},
};

use etcetera::{choose_base_strategy, BaseStrategy};

use crate::app::persistence::PersistedLines;

use self::connections::Connections;

pub mod clearable;
pub mod completion;
pub mod connections;
pub mod formatters;
pub mod history;
pub mod matchers;
pub mod persistence;
pub mod processing;
pub mod processors;

pub type Id = u64;

#[derive(Default)]
pub struct State {
    pub app_name: Option<String>,
    pub connections: Connections,
    pub persisted_output: HashMap<String, PersistedLines>,
}

#[derive(Default, Clone)]
pub struct LockableState(Arc<Mutex<State>>);

pub struct StatePath<'a> {
    pub key: &'a str,
    pub kind: &'a str,
}

impl LockableState {
    pub fn lock(&self) -> LockResult<MutexGuard<'_, State>> {
        self.0.lock()
    }

    pub async fn persisted_state_path(&self, path: StatePath<'_>) -> PathBuf {
        let app_name = self
            .lock()
            .unwrap()
            .app_name
            .clone()
            .unwrap_or_else(|| "kodachi".to_string());

        let strategy = choose_base_strategy().unwrap();
        let mut file_path = strategy.data_dir();
        file_path.push(app_name);
        file_path.push(path.key);
        file_path.push(path.kind);
        file_path
    }
}
