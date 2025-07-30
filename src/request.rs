pub mod initialize;
pub mod tool;

pub use initialize::Initialize;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum Request {
    #[serde(rename = "initialize")]
    Initialize { params: Initialize },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "tools/list")]
    ToolsList,
    #[serde(rename = "tools/call")]
    ToolsCall { params: tool::Call },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    jsonrpc: String,
    pub id: u64,
    #[serde(flatten)]
    pub request: Request,
}

impl Request {
    pub fn stamp(self, id: u64) -> Message {
        Message {
            jsonrpc: crate::JSONRPC.to_owned(),
            id,
            request: self,
        }
    }
}

impl From<Initialize> for Request {
    fn from(initialize: Initialize) -> Self {
        Self::Initialize { params: initialize }
    }
}

impl From<tool::Call> for Request {
    fn from(call: tool::Call) -> Self {
        Self::ToolsCall { params: call }
    }
}
