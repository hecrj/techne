use serde::{Deserialize, Serialize};

use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    code: i64,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub error: Error,
}

impl Error {
    fn new(code: i64, message: String) -> Self {
        Self { code, message }
    }

    pub fn invalid_json(message: String) -> Self {
        Self::new(-32700, message)
    }

    pub fn method_not_found(method: String) -> Self {
        Self::new(-32601, format!("Unknown method: {method}"))
    }

    pub fn invalid_params(message: String) -> Self {
        Self::new(-32602, message)
    }

    pub fn stamp(self, id: Option<u64>) -> Message {
        Message {
            jsonrpc: crate::JSONRPC.to_owned(),
            id,
            error: self,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{message} ({code})",
            message = self.message,
            code = self.code
        )
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
