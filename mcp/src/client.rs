pub mod capabilities;
pub mod notification;
pub mod request;
pub mod response;

pub use capabilities::Capabilities;
pub use notification::Notification;
pub use request::Request;
pub use response::Response;

pub type Message<T = Response> = crate::Message<Request, Notification, T>;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub version: String,
}
