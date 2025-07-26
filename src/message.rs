use crate::{Notification, Request, Response};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message<T = serde_json::Value> {
    Request(Request<T>),
    Notification(Notification<T>),
    Response(Response<T>),
}
