use crate::error;
use crate::notification;
use crate::request;
use crate::{Error, Response};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message<T = serde_json::Value> {
    Request(request::Message),
    Notification(notification::Message),
    Response(Response<T>),
    Error(error::Message),
}

impl Message {
    pub fn deserialize(json: &[u8]) -> Result<Message, error::Message> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum JsonRpc {
            Message(Message),
            Other { method: String },
        }

        match serde_json::from_slice(json) {
            Ok(JsonRpc::Message(message)) => Ok(message),
            Ok(JsonRpc::Other { method }) => Err(Error::method_not_found(method).stamp(None)),
            Err(error) => Err(Error::invalid_json(error.to_string()).stamp(None)),
        }
    }
}
