pub mod transport;

mod stdio;

pub use stdio::Stdio;
pub use transport::Transport;

use crate::mcp;
use crate::mcp::client::request;
use crate::mcp::client::{Capabilities, Message, Notification, Request, Response};
use crate::mcp::server;
use crate::mcp::server::tool;

use sipper::{Straw, sipper};

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
        name: impl AsRef<str>,
        version: impl AsRef<str>,
        transport: impl Transport + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let mut connection = Connection {
            transport: Arc::new(transport),
            next_request: mcp::Id::default(),
        };

        let initialize = connection
            .request(request::Initialize {
                protocol_version: mcp::VERSION.to_owned(),
                capabilities: Capabilities {},
                client_info: mcp::Client {
                    name: name.as_ref().to_owned(),
                    title: None, // TODO
                    version: version.as_ref().to_owned(),
                },
            })
            .await?
            .response::<server::response::Initialize>()
            .await?;

        if initialize.result.protocol_version != mcp::VERSION {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "protocol mismatch (supported: {supported}, given: {given})",
                    supported = mcp::VERSION,
                    given = initialize.result.protocol_version,
                ),
            ));
        }

        connection.notify(Notification::Initialized).await?;

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

    pub async fn list_tools(&mut self) -> io::Result<Vec<server::Tool>> {
        let list = self.connection.request(Request::ToolsList).await?;

        let mcp::Response {
            result: server::response::ToolsList { tools },
            ..
        } = list.response().await?;

        Ok(tools)
    }

    pub fn call_tool(
        &mut self,
        name: impl AsRef<str>,
        arguments: serde_json::Value,
    ) -> impl Straw<tool::Outcome, Event, io::Error> {
        sipper(async move |mut sender| {
            let mut call = self
                .connection
                .request(Request::ToolsCall {
                    params: request::ToolCall {
                        name: name.as_ref().to_owned(),
                        arguments,
                    },
                })
                .await?;

            loop {
                match call.next().await? {
                    server::Message::Request(request) => {
                        sender
                            .send(Event::Request(request.id, request.payload))
                            .await;
                    }
                    server::Message::Notification(notification) => {
                        sender.send(Event::Notification(notification.payload)).await;
                    }
                    server::Message::Response(response) => {
                        return Ok(response.result);
                    }
                    server::Message::Error { error, .. } => {
                        log::warn!("{error}");
                    }
                }
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Notification(server::Notification),
    Request(mcp::Id, server::Request),
}

struct Connection {
    transport: Arc<dyn Transport + Send + Sync>,
    next_request: mcp::Id,
}

impl Connection {
    async fn request(&mut self, request: impl Into<Request>) -> io::Result<transport::Receiver> {
        let request = request.into();

        self.transport
            .send(Message::request(self.next_request.increment(), request))
            .await
    }

    #[allow(unused)]
    async fn notify(&self, notification: impl Into<Notification>) -> io::Result<()> {
        let notification = notification.into();

        self.transport
            .send(Message::notification(notification))
            .await?;

        Ok(())
    }

    #[allow(unused)]
    async fn response(&self, id: mcp::Id, response: Response) -> io::Result<()> {
        self.transport.send(Message::response(id, response)).await;

        Ok(())
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("next_request", &self.next_request) // TODO: Debug transport
            .finish()
    }
}

#[derive(Debug)]
pub struct Server {
    capabilities: server::Capabilities,
    information: mcp::Server,
}

impl Server {
    pub fn capabilities(&self) -> &server::Capabilities {
        &self.capabilities
    }

    pub fn information(&self) -> &mcp::Server {
        &self.information
    }
}
