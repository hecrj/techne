use crate::Payload;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default = "none")]
    pub params: Option<Payload<T>>,
}

fn none<T>() -> Option<T> {
    None
}
