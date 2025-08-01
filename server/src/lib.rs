pub use techne_mcp as mcp;

pub mod tool;
pub mod transport;

mod connection;
#[cfg(feature = "http")]
mod http;
mod stdio;

#[cfg(feature = "http")]
pub use http::Http;
pub use stdio::Stdio;
pub use tool::Tool;
pub use transport::Transport;

use crate::connection::{Connection, Receipt};
use crate::mcp::client;
use crate::mcp::server;
use crate::mcp::server::response::{self, Response};
use crate::transport::{Action, Channel};

use tokio::task;

use std::collections::BTreeMap;
use std::env;
use std::io;
use std::sync::Arc;

#[derive(Default)]
pub struct Server {
    name: String,
    version: String,
    tools: BTreeMap<String, Tool>,
}

impl Server {
    pub fn new(name: impl AsRef<str>, version: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            version: version.as_ref().to_owned(),
            tools: BTreeMap::new(),
        }
    }

    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.tools = tools
            .into_iter()
            .map(|tool| (tool.name.clone(), tool))
            .collect();

        self
    }

    pub async fn run(self, mut transport: impl Transport) -> io::Result<()> {
        let server = Arc::new(self);

        loop {
            let action = transport.accept().await?;

            match action {
                Action::Subscribe(channel) => {
                    let _ = channel.send(transport::Result::Reject);
                }
                Action::Handle(bytes, channel) => {
                    let server = server.clone();

                    drop(task::spawn(async move {
                        if let Err(error) = server.handle(bytes, channel).await {
                            log::error!("{error}");
                        }
                    }));
                }
                Action::Quit => return Ok(()),
            }
        }
    }

    async fn handle(&self, bytes: mcp::Bytes, channel: Channel) -> io::Result<()> {
        match client::Message::<mcp::Value>::deserialize(&bytes) {
            Ok(message) => match message {
                client::Message::Request(request) => {
                    self.serve(Connection::new(request.id, channel), request.payload)
                        .await
                }
                client::Message::Notification(notification) => {
                    self.deliver_notification(Receipt::new(channel), notification.payload)
                        .await
                }
                client::Message::Response(response) => {
                    self.deliver_response(Receipt::new(channel), response).await
                }
                client::Message::Error(error) => {
                    self.deliver_error(Receipt::new(channel), error).await
                }
            },
            Err(error) => {
                let bytes = mcp::Error::invalid_json(error.to_string()).serialize()?;
                let _ = channel.send(transport::Result::Send(bytes));

                Ok(())
            }
        }
    }

    async fn serve(&self, connection: Connection, request: client::Request) -> io::Result<()> {
        log::debug!("Serving {request:?}");

        match request {
            client::Request::Initialize { .. } => self.initialize(connection).await,
            client::Request::Ping => self.ping(connection).await,
            client::Request::ToolsList => self.list_tools(connection).await,
            client::Request::ToolsCall { params: call } => self.call_tool(connection, call).await,
        }
    }

    async fn initialize(&self, connection: Connection) -> io::Result<()> {
        use crate::mcp::server::capabilities::{self, Capabilities};

        connection
            .finish(response::Initialize {
                protocol_version: mcp::VERSION.to_owned(),
                capabilities: Capabilities {
                    tools: (!self.tools.is_empty()).then_some(capabilities::Tools {
                        list_changed: false, // TODO?
                    }),
                },
                server_info: mcp::Server {
                    name: self.name.clone(),
                    version: self.version.clone(),
                },
            })
            .await
    }

    async fn ping(&self, connection: Connection) -> io::Result<()> {
        connection.finish(Response::Ping {}).await
    }

    async fn list_tools(&self, connection: Connection) -> io::Result<()> {
        connection
            .finish(response::ToolsList {
                tools: self
                    .tools
                    .values()
                    .map(|tool| server::Tool {
                        name: tool.name.clone(),
                        title: None,
                        description: tool.description.clone(),
                        input_schema: tool.input().clone(),
                        output_schema: tool.output().cloned(),
                    })
                    .collect(),
            })
            .await
    }

    async fn call_tool(
        &self,
        mut connection: Connection,
        call: client::request::ToolCall,
    ) -> io::Result<()> {
        use futures::StreamExt;

        let Some(tool) = self.tools.get(&call.name) else {
            return connection
                .error(mcp::ErrorKind::invalid_params(format!(
                    "Unknown tool: {}",
                    &call.name
                )))
                .await;
        };

        let mut output = tool.call(call.arguments)?.boxed();

        while let Some(action) = output.next().await {
            match action {
                crate::tool::Action::Request(request) => connection.request(request).await?,
                crate::tool::Action::Notify(notification) => {
                    connection.notify(notification).await?
                }
                crate::tool::Action::Finish(outcome) => return connection.finish(outcome?).await,
            }
        }

        Ok(())
    }

    async fn deliver_notification(
        &self,
        receipt: Receipt,
        _notification: client::Notification,
    ) -> io::Result<()> {
        // TODO
        receipt.reject();

        Ok(())
    }

    async fn deliver_response(&self, receipt: Receipt, _response: mcp::Response) -> io::Result<()> {
        // TODO
        receipt.reject();

        Ok(())
    }

    async fn deliver_error(&self, receipt: Receipt, _error: mcp::Error) -> io::Result<()> {
        // TODO
        receipt.reject();

        Ok(())
    }
}

pub async fn transport(mut args: env::Args) -> io::Result<impl Transport> {
    enum HttpOrStdio {
        #[cfg(feature = "http")]
        Http(Http),
        Stdio(Stdio),
    }

    impl Transport for HttpOrStdio {
        fn accept(&mut self) -> impl Future<Output = io::Result<Action>> {
            use futures::FutureExt;

            match self {
                #[cfg(feature = "http")]
                HttpOrStdio::Http(http) => http.accept().boxed(),
                HttpOrStdio::Stdio(stdio) => stdio.accept().boxed(),
            }
        }
    }

    let _executable = args.next();

    let protocol = args.next();
    let protocol = protocol.as_deref();

    if protocol == Some("--http") {
        #[cfg(feature = "http")]
        {
            let address = args.next();
            let address = address.as_deref().unwrap_or("127.0.0.1:8080");

            let rest = args.next();

            if let Some(rest) = rest {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown argument: {rest}"),
                ));
            }

            return Ok(HttpOrStdio::Http(Http::bind(address).await?));
        }

        #[cfg(not(feature = "http"))]
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Streamable HTTP is not supported for this server"),
        ));
    }

    if let Some(protocol) = protocol {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown argument: {protocol}"),
        ));
    }

    Ok(HttpOrStdio::Stdio(Stdio::current()))
}
