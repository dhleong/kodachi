use std::hash::Hash;

use ritelinked::LinkedHashSet;
use serde::Deserialize;

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

    pub fn iter(&self) -> ritelinked::linked_hash_set::Iter<T> {
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
