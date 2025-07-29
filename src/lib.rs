pub mod client;
pub mod server;
pub mod tool;

mod content;
mod error;
mod log;
mod message;
mod notification;
mod request;
mod response;
mod schema;

pub use client::Client;
pub use content::Content;
pub use schema::Schema;
pub use server::Server;
pub use tool::Tool;

use error::Error;
use message::Message;
use notification::Notification;
use request::Request;
use response::Response;

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const JSONRPC: &str = "2.0";
