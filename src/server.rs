pub mod transport;

mod stdio;

pub use stdio::Stdio;
pub use transport::Transport;

#[cfg(feature = "server-http")]
mod http;

#[cfg(feature = "server-http")]
pub use http::Http;

use crate::Tool;
use crate::request;
use crate::response;
use crate::server::transport::{Action, Connection, Decision};

use futures::StreamExt;
use tokio::task;

use std::collections::BTreeMap;
use std::env;
use std::io;
use std::sync::Arc;

#[derive(Default)]
pub struct Server {
    tools: BTreeMap<String, Tool>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.tools = tools
            .into_iter()
            .map(|tool| (tool.name.clone().into_owned(), tool))
            .collect();

        self
    }

    pub async fn run(self, mut transport: impl Transport) -> io::Result<()> {
        let server = Arc::new(self);

        loop {
            let connect = transport.connect().await;

            let connections = match connect {
                Ok(connections) => connections.boxed(),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                    return Ok(());
                }
                Err(error) => {
                    log::error!("{error}");

                    return Err(error);
                }
            };

            let mut connections = connections.boxed();
            let server = server.clone();

            drop(task::spawn(async move {
                while let Some(action) = connections.next().await {
                    if let Err(error) = server.serve(action).await {
                        log::error!("{error}");
                    }
                }

                Ok::<_, io::Error>(())
            }))
        }
    }

    pub async fn run_with_args(self, mut args: env::Args) -> io::Result<()> {
        let _executable = args.next();

        let protocol = args.next();
        let protocol = protocol.as_deref();

        if protocol == Some("--http") {
            #[cfg(feature = "server-http")]
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

                return self.run(Http::bind(address).await?).await;
            }

            #[cfg(not(feature = "server-http"))]
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

        self.run(Stdio::current()).await
    }

    pub async fn serve<C: Connection, D: Decision>(&self, action: Action<C, D>) -> io::Result<()> {
        match action {
            Action::Request(connection, request) => {
                log::debug!("Serving {request:?}");

                match request.method.as_str() {
                    "initialize" => self.initialize(connection).await,
                    "ping" => self.ping(connection).await,
                    "tools/list" => self.list_tools(connection).await,
                    "tools/call" => {
                        let call = request.deserialize()?;

                        self.call_tool(connection, call).await
                    }
                    _ => connection.reject().await,
                }
            }
            // TODO: Out of channel deliveries
            Action::Deliver(decision, _deliver) => decision.reject().await,
        }
    }

    async fn initialize(&self, connection: impl Connection) -> io::Result<()> {
        use response::initialize;

        connection
            .finish(response::Initialize {
                protocol_version: "2025-06-18".to_owned(),
                capabilities: initialize::Capabilities {
                    tools: (!self.tools.is_empty()).then_some(initialize::Tools {
                        list_changed: false, // TODO?
                    }),
                },
                server_info: initialize::ServerInfo {
                    name: "techne-server".to_owned(),
                    version: env!("CARGO_PKG_VERSION").to_owned(),
                },
            })
            .await
    }

    async fn ping(&self, connection: impl Connection) -> io::Result<()> {
        connection.finish(serde_json::json!({})).await
    }

    async fn list_tools(&self, connection: impl Connection) -> io::Result<()> {
        use response::tool;

        connection
            .finish(tool::List {
                tools: self
                    .tools
                    .values()
                    .map(|tool| response::Tool {
                        name: tool.name.clone().into_owned(),
                        title: None,
                        description: tool.description.clone().into_owned(),
                        input_schema: tool.input().clone(),
                        output_schema: tool.output().cloned(),
                    })
                    .collect(),
            })
            .await
    }

    async fn call_tool(
        &self,
        mut connection: impl Connection,
        call: request::tool::Call,
    ) -> io::Result<()> {
        use futures::StreamExt;

        let Some(tool) = self.tools.get(&call.name) else {
            return connection.reject().await;
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
}
