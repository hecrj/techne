pub mod transport;

mod stdio;

pub use stdio::Stdio;
pub use transport::Transport;

use crate::request;
use crate::response;
use crate::tool;
use crate::{Message, Notification, Request, Response};

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
            request: 0,
        };

        let initialize = connection
            .request(request::Initialize {
                protocol_version: crate::PROTOCOL_VERSION.to_owned(),
                capabilities: request::initialize::Capabilities {},
                client_info: request::initialize::ClientInfo {
                    name: name.as_ref().to_owned(),
                    title: None, // TODO
                    version: version.as_ref().to_owned(),
                },
            })
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

    pub async fn list_tools(&mut self) -> io::Result<Vec<response::Tool>> {
        let list = self.connection.request(Request::ToolsList).await?;

        let Response {
            result: response::tool::List { tools },
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
                    params: request::tool::Call {
                        name: name.as_ref().to_owned(),
                        arguments,
                    },
                })
                .await?;

            loop {
                match call.next().await? {
                    Message::Request(request) => {
                        sender.send(Event::Request(request)).await;
                    }
                    Message::Notification(message) => {
                        sender.send(Event::Notification(message.notification)).await;
                    }
                    Message::Response(response) => {
                        return Ok(response.result);
                    }
                    Message::Error(error) => {
                        log::warn!("{error}");
                    }
                }
            }
        })
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Notification(Notification),
    Request(request::Message),
}

struct Connection {
    transport: Arc<dyn Transport + Send + Sync>,
    request: u64,
}

impl Connection {
    async fn request(&mut self, request: impl Into<Request>) -> io::Result<transport::Receiver> {
        let request = request.into();

        let id = self.request;
        self.request += 1;

        self.transport
            .send(Message::Request(request.stamp(id)))
            .await
    }

    #[allow(unused)]
    async fn notify(&self, notification: impl Into<Notification>) -> io::Result<()> {
        let notification = notification.into();

        self.transport
            .send(Message::Notification(notification.stamp()))
            .await?;

        Ok(())
    }

    #[allow(unused)]
    async fn response(&self, response: Response) -> io::Result<()> {
        self.transport.send(Message::Response(response)).await;

        Ok(())
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
