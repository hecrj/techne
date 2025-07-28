pub mod initialize;
pub mod tool;

pub use initialize::Initialize;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request<T = serde_json::Value> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default = "none")]
    pub params: Option<T>,
}

impl Request {
    pub fn deserialize<T: DeserializeOwned>(self) -> serde_json::Result<T> {
        serde_json::from_value(self.params.unwrap_or(serde_json::Value::Null))
    }
}

fn none<T>() -> Option<T> {
    None
}
