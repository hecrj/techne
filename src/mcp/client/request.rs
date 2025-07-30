use crate::mcp::client::{self, Client};

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
    ToolsCall { params: ToolCall },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Initialize {
    pub protocol_version: String,
    pub capabilities: client::Capabilities,
    pub client_info: Client,
}

impl From<Initialize> for Request {
    fn from(initialize: Initialize) -> Self {
        Self::Initialize { params: initialize }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

impl From<ToolCall> for Request {
    fn from(call: ToolCall) -> Self {
        Self::ToolsCall { params: call }
    }
}
