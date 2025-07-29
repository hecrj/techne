use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification<T = serde_json::Value> {
    jsonrpc: String,
    method: String,
    #[serde(default = "none")]
    params: Option<T>,
}

impl<T> Notification<T> {
    pub fn serialize(self) -> serde_json::Result<Notification>
    where
        T: Serialize,
    {
        Ok(Notification {
            jsonrpc: self.jsonrpc,
            method: self.method,
            params: self.params.map(serde_json::to_value).transpose()?,
        })
    }
}

fn none<T>() -> Option<T> {
    None
}
