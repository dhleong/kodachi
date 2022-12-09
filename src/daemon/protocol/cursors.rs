use serde::{Deserialize, Serialize};

use crate::app::Id;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryCursor {
    pub limit: usize,
    pub offset: usize,
    pub version: Id,
    pub initial_content: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct JsonSerializable {
    pub limit: usize,
    pub offset: usize,
    pub version: Id,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_content: Option<String>,
}

impl Serialize for HistoryCursor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = JsonSerializable {
            limit: self.limit,
            offset: self.offset,
            version: self.version,
            initial_content: self.initial_content.clone(),
        };
        match serde_json::to_string(&s) {
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
        match serde_json::from_str::<JsonSerializable>(&encoded) {
            Ok(cursor) => Ok(HistoryCursor {
                limit: cursor.limit,
                offset: cursor.offset,
                version: cursor.version,
                initial_content: cursor.initial_content,
            }),
            Err(err) => panic!("{:?}", err),
        }
    }
}
