use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Content<T = serde_json::Value> {
    #[serde(rename = "content")]
    Unstructured(Vec<Unstructured>),
    #[serde(rename = "structuredContent")]
    Structured(T),
}

impl<T> From<Unstructured> for Content<T> {
    fn from(content: Unstructured) -> Self {
        Content::Unstructured(vec![content])
    }
}

impl<T> From<String> for Content<T> {
    fn from(text: String) -> Self {
        Content::Unstructured(vec![Unstructured::Text { text }])
    }
}

impl<T> From<u32> for Content<T> {
    fn from(number: u32) -> Self {
        Content::Unstructured(vec![Unstructured::Text {
            text: number.to_string(),
        }])
    }
}

impl From<serde_json::Value> for Content {
    fn from(json: serde_json::Value) -> Self {
        Content::Structured(json)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Unstructured {
    Text {
        text: String,
    },
    Image {
        data: Base64,
        mime_type: String,
    },
    Audio {
        data: Base64,
        mime_type: String,
    },
    ResourceLink {
        uri: String,
        name: String,
        description: String,
        mime_type: String,
    },
    Resource {
        uri: String,
        title: String,
        mime_type: String,
        text: String,
    },
}

// TODO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Base64(String);
