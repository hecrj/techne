use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Schema {
    Object {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        properties: BTreeMap<String, Schema>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        required: Vec<String>,
    },
    String {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Integer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Number {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Boolean {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Array {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        items: Option<Box<Schema>>,
    },
    Null,
}
