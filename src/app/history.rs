use std::{
    hash::Hash,
    io::{self},
    path::Path,
};

use ritelinked::LinkedHashSet;
use serde::Deserialize;
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt as _, AsyncWriteExt, BufReader},
};

const DEFAULT_HISTORY_CAPACITY: usize = 10000;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum HistoryScrollDirection {
    Older,
    Newer,
}

pub struct History<T> {
    max_entries: usize,
    entries: LinkedHashSet<T>,
    version: u64,
}

impl<T: Eq + Hash> Default for History<T> {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_HISTORY_CAPACITY)
    }
}

impl<T: Eq + Hash> History<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            max_entries: capacity,
            entries: LinkedHashSet::default(),
            version: 0,
        }
    }

    pub fn insert(&mut self, entry: T) {
        self.insert_many(vec![entry]);
    }

    pub fn insert_many<I: IntoIterator<Item = T>>(&mut self, entries: I) {
        self.entries.extend(entries);

        if let Some(overage) = self.entries.len().checked_sub(self.max_entries) {
            for _ in 0..overage {
                self.entries.pop_front();
            }
        }

        self.on_modified();
    }

    pub fn iter(&self) -> ritelinked::linked_hash_set::Iter<'_, T> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    fn on_modified(&mut self) {
        let (result, _overflowed) = self.version.overflowing_add(1);
        self.version = result;
    }
}

impl History<String> {
    pub async fn load(path: &Path) -> io::Result<Self> {
        Self::load_with_capacity(path, DEFAULT_HISTORY_CAPACITY).await
    }

    pub async fn load_with_capacity(path: &Path, capacity: usize) -> io::Result<Self> {
        let mut instance = Self::with_capacity(capacity);
        let file = match File::open(&path).await {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // New file
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                return Ok(instance);
            }
            Err(err) => return Err(err),
        };
        let reader = BufReader::new(file);

        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            instance.insert(line);
        }
        Ok(instance)
    }

    pub async fn save_to(&self, path: &Path) -> io::Result<()> {
        let pending_path = path.with_extension(".pending");
        let mut file = File::create(&pending_path).await?;
        for line in self.iter() {
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }
        file.flush().await?;
        tokio::fs::rename(&pending_path, path).await?;
        Ok(())
    }
}

impl<'a, T> IntoIterator for &'a History<T> {
    type Item = &'a T;

    type IntoIter = ritelinked::linked_hash_set::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl<T> IntoIterator for History<T> {
    type Item = T;

    type IntoIter = ritelinked::linked_hash_set::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_overflow_test() {
        let mut history = History {
            version: u64::MAX,
            ..Default::default()
        };
        history.insert("String");
        assert_eq!(history.version, 0);
    }
}
