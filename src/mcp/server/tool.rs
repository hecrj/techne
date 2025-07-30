use crate::mcp::Schema;
use crate::mcp::server::content::{self, Content};

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
#[serde(rename_all = "camelCase")]
pub struct Outcome<T = serde_json::Value> {
    #[serde(flatten)]
    content: Content<T>,
    is_error: bool,
}

impl<T> Outcome<T> {
    pub async fn serialize(self) -> serde_json::Result<Outcome>
    where
        T: Serialize,
    {
        Ok(Outcome {
            content: match self.content {
                Content::Unstructured(content) => Content::Unstructured(content),
                Content::Structured(content) => {
                    Content::Structured(serde_json::to_value(&content)?)
                }
            },
            is_error: self.is_error,
        })
    }
}

pub trait IntoOutcome {
    type Content;

    fn into_outcome(self) -> Outcome<Self::Content>;
}

impl<T> IntoOutcome for T
where
    T: Into<Content<T>>,
{
    type Content = T;

    fn into_outcome(self) -> Outcome<Self::Content> {
        Outcome {
            content: self.into(),
            is_error: false,
        }
    }
}

impl<T, E> IntoOutcome for Result<T, E>
where
    T: Into<Content<T>>,
    E: std::error::Error,
{
    type Content = T;

    fn into_outcome(self) -> Outcome<T> {
        match self {
            Ok(value) => Outcome {
                content: value.into(),
                is_error: false,
            },
            Err(error) => Outcome {
                content: Content::Unstructured(vec![content::Unstructured::Text {
                    text: error.to_string(),
                }]),
                is_error: false,
            },
        }
    }
}
