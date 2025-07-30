pub mod capabilities;
pub mod content;
pub mod notification;
pub mod request;
pub mod response;
pub mod tool;

pub use capabilities::Capabilities;
pub use content::Content;
pub use notification::Notification;
pub use request::Request;
pub use response::Response;
pub use tool::Tool;

pub type Message<T = Response> = crate::mcp::Message<Request, Notification, T>;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub version: String,
}
