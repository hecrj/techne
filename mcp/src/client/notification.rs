use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Notification {
    #[serde(rename = "initialized")]
    Initialized,
}
