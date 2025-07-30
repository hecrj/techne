use crate::mcp;
use crate::mcp::client::Message;
use crate::mcp::server;

use futures::StreamExt;
use futures::channel::mpsc;
use serde::de::DeserializeOwned;

use std::io;

pub trait Transport {
    fn listen(&self) -> Task;

    fn send(&self, message: Message) -> Task;
}

pub type Task = futures::future::BoxFuture<'static, io::Result<Receiver>>;

pub struct Receiver {
    raw: mpsc::Receiver<io::Result<server::Message<serde_json::Value>>>,
}

impl Receiver {
    pub fn new(raw: mpsc::Receiver<io::Result<server::Message<serde_json::Value>>>) -> Self {
        Self { raw }
    }

    pub(crate) async fn next<T: DeserializeOwned>(&mut self) -> io::Result<server::Message<T>> {
        let Some(message) = self.raw.next().await.transpose()? else {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "stream was closed by peer",
            ));
        };

        Ok(message.decode()?)
    }

    pub(crate) async fn response<T: DeserializeOwned>(mut self) -> io::Result<mcp::Response<T>> {
        loop {
            if let server::Message::Response(response) = self.next().await? {
                return Ok(response);
            }
        }
    }
}
