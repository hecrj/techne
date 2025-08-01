use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Tools>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    #[serde(default, skip_serializing_if = "is_false")]
    pub list_changed: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}
