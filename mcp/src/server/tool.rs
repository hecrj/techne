use crate::Schema;
use crate::server::content::{self, Content};

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
pub struct Response<T = serde_json::Value> {
    #[serde(flatten)]
    pub content: Content<T>,
    pub is_error: bool,
}

impl<T> Response<T> {
    pub async fn serialize(self) -> serde_json::Result<Response>
    where
        T: Serialize,
    {
        Ok(Response {
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

pub trait IntoResponse {
    type Content;

    fn into_outcome(self) -> Response<Self::Content>;
}

impl<T> IntoResponse for T
where
    T: Into<Content<T>>,
{
    type Content = T;

    fn into_outcome(self) -> Response<Self::Content> {
        Response {
            content: self.into(),
            is_error: false,
        }
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: Into<Content<T>>,
    E: std::error::Error,
{
    type Content = T;

    fn into_outcome(self) -> Response<T> {
        match self {
            Ok(value) => Response {
                content: value.into(),
                is_error: false,
            },
            Err(error) => Response {
                content: Content::Unstructured(vec![content::Unstructured::Text {
                    text: error.to_string(),
                }]),
                is_error: true,
            },
        }
    }
}
