use crate::Payload;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T> {
    jsonrpc: String,
    id: u64,
    result: Payload<T>,
}

impl<T> Response<T> {
    pub fn new(id: u64, result: T) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result: Payload::new(result),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Initialization {
    pub capabilities: Capabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}
