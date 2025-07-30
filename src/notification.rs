use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "lowercase")]
pub enum Notification {
    Initialized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    jsonrpc: String,
    #[serde(flatten)]
    pub notification: Notification,
}

impl Notification {
    pub fn stamp(self) -> Message {
        Message {
            jsonrpc: crate::JSONRPC.to_owned(),
            notification: self,
        }
    }
}
