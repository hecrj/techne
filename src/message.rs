use crate::{Error, Notification, Request, Response};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message<T = serde_json::Value> {
    Request(Request<T>),
    Notification(Notification<T>),
    Response(Response<T>),
    Error(Error<T>),
}

impl<T> Message<T> {
    pub fn serialize(self) -> serde_json::Result<Message>
    where
        T: Serialize,
    {
        Ok(match self {
            Message::Request(request) => Message::Request(request.serialize()?),
            Message::Notification(notification) => Message::Notification(notification.serialize()?),
            Message::Response(response) => Message::Response(response.serialize()?),
            Message::Error(error) => Message::Error(error.serialize()?),
        })
    }
}
