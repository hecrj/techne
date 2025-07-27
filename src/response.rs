pub mod initialize;
pub mod tool;

pub use initialize::Initialize;
pub use tool::Tool;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T = serde_json::Value> {
    jsonrpc: String,
    id: u64,
    result: T,
}

impl<T> Response<T> {
    pub fn new(id: u64, result: T) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result,
        }
    }
}
