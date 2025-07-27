use crate::Payload;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification<T = serde_json::Value> {
    jsonrpc: String,
    method: String,
    #[serde(default = "none")]
    params: Option<Payload<T>>,
}

fn none<T>() -> Option<T> {
    None
}
