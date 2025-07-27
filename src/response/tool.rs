use crate::Schema;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: String,
    pub input_schema: Schema,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Schema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub tools: Vec<Tool>,
}
