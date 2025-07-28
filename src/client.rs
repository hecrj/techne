pub mod transport;

mod stdio;

pub use stdio::Stdio;
pub use transport::Transport;

use crate::request;
use crate::response;
use crate::{Message, Notification, Request, Response};

use futures::FutureExt;
use serde::Serialize;

use std::borrow::Cow;
use std::fmt;
use std::io;
use std::sync::Arc;

#[derive(Debug)]
pub struct Client {
    connection: Connection,
    server: Server,
}

impl Client {
    pub async fn new(
        name: impl Into<Cow<'_, str>>,
        version: impl Into<Cow<'_, str>>,
        transport: impl Transport + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let mut connection = Connection {
            transport: Arc::new(transport),
            request: 0,
        };

        let initialize = connection
            .request(
                "initialize",
                request::Initialize {
                    protocol_version: crate::PROTOCOL_VERSION.to_owned(),
                    capabilities: request::initialize::Capabilities {},
                    client_info: request::initialize::ClientInfo {
                        name: name.into().into_owned(),
                        title: None, // TODO
                        version: version.into().into_owned(),
                    },
                },
            )
            .await?
            .response::<response::Initialize>()
            .await?;

        if initialize.result.protocol_version != crate::PROTOCOL_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "protocol mismatch (supported: {supported}, given: {given})",
                    supported = crate::PROTOCOL_VERSION,
                    given = initialize.result.protocol_version,
                ),
            ));
        }

        Ok(Self {
            connection,
            server: Server {
                capabilities: initialize.result.capabilities,
                information: initialize.result.server_info,
            },
        })
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    pub async fn list_tools(&mut self) -> io::Result<Vec<response::Tool>> {
        let mut list = self
            .connection
            .request("tools/list", serde_json::json!({}))
            .await?;

        let Response {
            result: response::tool::List { tools },
            ..
        } = list.response().await?;

        Ok(tools)
    }
}

struct Connection {
    transport: Arc<dyn Transport + Send + Sync>,
    request: u64,
}

impl Connection {
    fn request<T: Serialize + Send + 'static>(
        &mut self,
        method: &'static str,
        params: T,
    ) -> transport::Task {
        let transport = self.transport.clone();
        let id = self.request;

        self.request += 1;

        async move {
            transport
                .send(Message::Request(Request {
                    jsonrpc: crate::JSONRPC.to_owned(),
                    id,
                    method: method.to_owned(),
                    params: Some(serde_json::to_value(params)?),
                }))
                .await
        }
        .boxed()
    }

    #[allow(unused)]
    fn notify(&self, notification: Notification) -> transport::Task {
        self.transport.send(Message::Notification(notification))
    }

    #[allow(unused)]
    fn response(&self, response: Response) -> transport::Task {
        self.transport.send(Message::Response(response))
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("request", &self.request) // TODO: Debug transport
            .finish()
    }
}

#[derive(Debug)]
pub struct Server {
    capabilities: response::initialize::Capabilities,
    information: response::initialize::ServerInfo,
}

impl Server {
    pub fn capabilities(&self) -> &response::initialize::Capabilities {
        &self.capabilities
    }

    pub fn information(&self) -> &response::initialize::ServerInfo {
        &self.information
    }
}
