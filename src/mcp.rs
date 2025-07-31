pub mod client;
pub mod server;

mod schema;

pub use client::Client;
pub use schema::Schema;
pub use server::Server;

pub use bytes::Bytes;
pub use serde::de::IgnoredAny as Ignored;
pub use serde_json::Value;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const VERSION: &str = "2025-06-18";
pub const JSONRPC: &str = "2.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message<R, N, T = Value> {
    Request(Request<R>),
    Notification(Notification<N>),
    Response(Response<T>),
    Error(Error),
}

impl<R, N, T> Message<R, N, T> {
    pub fn request(id: Id, payload: R) -> Self {
        Self::Request(Request::new(id, payload))
    }

    pub fn notification(payload: N) -> Self {
        Self::Notification(Notification::new(payload))
    }

    pub fn response(id: Id, result: T) -> Self {
        Self::Response(Response::new(id, result))
    }

    pub fn error(id: Option<Id>, payload: ErrorKind) -> Self {
        Self::Error(Error::new(id, payload))
    }

    pub fn deserialize(json: &[u8]) -> Result<Self, Error>
    where
        R: DeserializeOwned,
        N: DeserializeOwned,
        T: DeserializeOwned,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum JsonRpc<R, N, T> {
            Message(Message<R, N, T>),
            Other { method: String },
        }

        match serde_json::from_slice(json) {
            Ok(JsonRpc::Message(message)) => Ok(message),
            Ok(JsonRpc::Other { method }) => Err(Error::method_not_found(method)),
            Err(error) => Err(Error::invalid_json(error.to_string())),
        }
    }
}

impl<R, N> Message<R, N> {
    pub fn decode<A: DeserializeOwned>(self) -> serde_json::Result<Message<R, N, A>> {
        Ok(match self {
            Self::Request(message) => Message::Request(message),
            Self::Notification(notification) => Message::Notification(notification),
            Self::Response(response) => Message::Response(Response::new(
                response.id,
                serde_json::from_value(response.result)?,
            )),
            Self::Error(error) => Message::Error(error),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request<T> {
    jsonrpc: String,
    pub id: Id,
    #[serde(flatten)]
    pub payload: T,
}

impl<T> Request<T> {
    pub(crate) fn new(id: Id, payload: T) -> Self {
        Self {
            jsonrpc: JSONRPC.to_owned(),
            id,
            payload,
        }
    }

    pub fn serialize(&self) -> serde_json::Result<Bytes>
    where
        T: Serialize,
    {
        serde_json::to_vec(self).map(Bytes::from_owner)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T = serde_json::Value> {
    jsonrpc: String,
    pub id: Id,
    pub result: T,
}

impl<T> Response<T> {
    pub(crate) fn new(id: Id, result: T) -> Self {
        Self {
            jsonrpc: JSONRPC.to_owned(),
            id,
            result,
        }
    }

    pub fn serialize(&self) -> serde_json::Result<Bytes>
    where
        T: Serialize,
    {
        serde_json::to_vec(self).map(Bytes::from_owner)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification<T> {
    jsonrpc: String,
    #[serde(flatten)]
    pub payload: T,
}

impl<T> Notification<T> {
    pub(crate) fn new(payload: T) -> Self {
        Self {
            jsonrpc: JSONRPC.to_owned(),
            payload,
        }
    }

    pub fn serialize(&self) -> serde_json::Result<Bytes>
    where
        T: Serialize,
    {
        serde_json::to_vec(self).map(Bytes::from_owner)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    #[serde(rename = "error")]
    payload: ErrorKind,
}

impl Error {
    pub fn new(id: Option<Id>, payload: ErrorKind) -> Self {
        Self {
            jsonrpc: JSONRPC.to_owned(),
            id,
            payload,
        }
    }

    pub fn method_not_found(method: String) -> Self {
        Self::new(
            None,
            ErrorKind::new(-32601, format!("Unknown method: {method}")),
        )
    }

    pub fn invalid_json(message: String) -> Self {
        Self::new(None, ErrorKind::new(-32700, message))
    }

    pub fn serialize(&self) -> serde_json::Result<Bytes> {
        serde_json::to_vec(self).map(Bytes::from_owner)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorKind {
    code: i64,
    message: String,
}

impl ErrorKind {
    fn new(code: i64, message: String) -> Self {
        Self { code, message }
    }

    pub fn invalid_params(message: String) -> Self {
        Self::new(-32602, message)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.payload.fmt(f)
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{message} ({code})",
            message = self.message,
            code = self.code
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Id(u64);

impl Id {
    pub fn increment(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}
