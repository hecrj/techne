use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Initialize {
    pub protocol_version: String,
    pub capabilities: Capabilities,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    // TODO: Roots
    // TODO: Sampling
    // TODO: Elicitation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub version: String,
}
