use crate::mcp::server::tool;
use crate::mcp::server::{Capabilities, Server, Tool};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Response {
    Initialize(Initialize),
    ToolsList(ToolsList),
    ToolsCall(tool::Outcome),
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Initialize {
    pub protocol_version: String,
    pub capabilities: Capabilities,
    pub server_info: Server,
}

impl From<Initialize> for Response {
    fn from(response: Initialize) -> Self {
        Self::Initialize(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsList {
    pub tools: Vec<Tool>,
}

impl From<ToolsList> for Response {
    fn from(response: ToolsList) -> Self {
        Self::ToolsList(response)
    }
}

impl From<tool::Outcome> for Response {
    fn from(response: tool::Outcome) -> Self {
        Self::ToolsCall(response)
    }
}
