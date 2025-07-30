pub mod initialize;
pub mod tool;

pub use initialize::Initialize;
pub use tool::Tool;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T = serde_json::Value> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: T,
}

impl<T> Response<T> {
    pub fn serialize(self) -> serde_json::Result<Response>
    where
        T: Serialize,
    {
        Ok(Response {
            jsonrpc: self.jsonrpc,
            id: self.id,
            result: serde_json::to_value(self.result)?,
        })
    }
}
