use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error<T = serde_json::Value> {
    pub jsonrpc: String,
    pub id: u64,
    pub error: Body<T>,
}

impl<T> Error<T> {
    pub fn serialize(self) -> serde_json::Result<Error>
    where
        T: Serialize,
    {
        Ok(Error {
            jsonrpc: self.jsonrpc,
            id: self.id,
            error: Body {
                code: self.error.code,
                message: self.error.message,
                data: self.error.data.map(serde_json::to_value).transpose()?,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<T = serde_json::Value> {
    code: i64,
    message: String,
    #[serde(flatten)]
    data: Option<T>,
}

pub fn method_not_found(message: String) -> Body {
    empty(-32601, message)
}

pub fn invalid_params(message: String) -> Body {
    empty(-32602, message)
}

fn empty(code: i64, message: String) -> Body {
    Body {
        code,
        message,
        data: None,
    }
}
