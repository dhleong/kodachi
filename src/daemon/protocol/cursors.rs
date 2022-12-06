use serde::{Deserialize, Serialize};

use crate::app::Id;

pub struct HistoryCursor {
    pub limit: usize,
    pub offset: usize,
    pub version: Id,
}

impl Serialize for HistoryCursor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match serde_json::to_string(self) {
            Ok(as_str) => serializer.serialize_str(&as_str),
            Err(err) => panic!("Unable to serialize cursor: {:?}", err),
        }
    }
}

impl<'de> Deserialize<'de> for HistoryCursor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        match serde_json::from_str::<HistoryCursor>(&encoded) {
            Ok(cursor) => Ok(cursor),
            Err(err) => panic!("{:?}", err),
        }
    }
}
