use crate::{Message, Response};

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
    raw: mpsc::Receiver<io::Result<Message>>,
}

impl Receiver {
    pub fn new(raw: mpsc::Receiver<io::Result<Message>>) -> Self {
        Self { raw }
    }

    pub(crate) async fn next<T: DeserializeOwned>(&mut self) -> io::Result<Message<T>> {
        let Some(message) = self.raw.next().await.transpose()? else {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "stream was closed by peer",
            ));
        };

        Ok(match message {
            Message::Request(message) => Message::Request(message),
            Message::Notification(notification) => Message::Notification(notification),
            Message::Response(response) => Message::Response(Response {
                jsonrpc: crate::JSONRPC.to_owned(),
                id: response.id,
                result: serde_json::from_value(response.result)?,
            }),
            Message::Error(error) => Message::Error(error),
        })
    }

    pub(crate) async fn response<T: DeserializeOwned>(mut self) -> io::Result<Response<T>> {
        loop {
            if let Message::Response(response) = self.next().await? {
                return Ok(response);
            }
        }
    }
}
