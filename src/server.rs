#[cfg(feature = "http")]
mod http;

pub use http::Http;

use crate::response;
use crate::{Message, Notification, Request, Response};

use futures::{Stream, StreamExt};
use serde::Serialize;
use tokio::task;

use std::io;
use std::sync::Arc;

#[derive(Default)]
pub struct Server;

impl Server {
    pub fn new() -> Self {
        Self
    }

    pub async fn serve(&self, connection: impl Connection, action: Action) -> io::Result<()> {
        match action {
            Action::Listen => connection.reject().await,
            Action::Talk(message) => match dbg!(message) {
                Message::Request(request) => match request.method.as_str() {
                    "initialize" => {
                        connection
                            .finish(Response::new(
                                request.id,
                                response::Initialization {
                                    capabilities: response::Capabilities {},
                                    server_info: response::ServerInfo {
                                        name: "techne-server".to_owned(),
                                        version: env!("CARGO_PKG_VERSION").to_owned(),
                                    },
                                },
                            ))
                            .await
                    }
                    _ => connection.reject().await,
                },
                Message::Notification(_notification) => connection.reject().await,
                Message::Response(_response) => connection.reject().await,
            },
        }
    }

    pub async fn run(self, mut transport: impl Transport + 'static) -> io::Result<()> {
        let server = Arc::new(self);

        loop {
            let Ok(connections) = transport
                .connect()
                .await
                .inspect_err(|error| log::error!("{error}"))
            else {
                continue;
            };

            let mut connections = connections.boxed();
            let server = server.clone();

            drop(task::spawn(async move {
                while let Some((connection, action)) = connections.next().await {
                    if let Err(error) = server.serve(connection, action).await {
                        log::error!("{error}");
                    }
                }

                Ok::<_, io::Error>(())
            }))
        }
    }
}

pub enum Action {
    Listen,
    Talk(Message),
}

pub trait Transport {
    type Connection: Connection + Send + 'static;

    fn connect(
        &mut self,
    ) -> impl Future<Output = io::Result<impl Stream<Item = (Self::Connection, Action)> + Send + 'static>>;
}

pub trait Connection {
    fn accept(self) -> impl Future<Output = io::Result<()>> + Send;

    fn reject(self) -> impl Future<Output = io::Result<()>> + Send;

    fn request<T: Serialize>(
        &mut self,
        message: Request<T>,
    ) -> impl Future<Output = io::Result<()>> + Send;

    fn notify<T: Serialize>(
        &mut self,
        message: Notification<T>,
    ) -> impl Future<Output = io::Result<()>> + Send;

    fn finish<T: Serialize>(
        self,
        response: Response<T>,
    ) -> impl Future<Output = io::Result<()>> + Send;
}
