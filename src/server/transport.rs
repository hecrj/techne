use crate::mcp::client;
use crate::mcp::server::{Message, Notification, Request, Response};
use crate::mcp::{self, Error, Id};

use futures::SinkExt;
use futures::channel::mpsc;
use futures::channel::oneshot;

use std::io;

pub trait Transport {
    fn accept(&mut self) -> impl Future<Output = io::Result<Action>>;
}

pub enum Action {
    Request(Connection, client::Request),
    Notify(Receipt, client::Notification),
    Respond(Receipt, mcp::Response),
    Quit,
}

#[derive(Debug, Clone)]
pub struct Connection {
    id: Id,
    sender: mpsc::Sender<Message>,
    total_requests: Id,
}

impl Connection {
    pub fn new(id: Id, sender: mpsc::Sender<Message>) -> Self {
        Self {
            id,
            sender,
            total_requests: Id::default(),
        }
    }

    pub async fn request(&mut self, request: Request) -> io::Result<()> {
        let id = self.total_requests.increment();

        self.send(Message::request(id, request)).await
    }

    pub async fn notify(&mut self, notification: Notification) -> io::Result<()> {
        self.send(Message::notification(notification)).await
    }

    pub async fn error(mut self, error: Error) -> io::Result<()> {
        self.send(Message::error(Some(self.id), error)).await
    }

    pub async fn finish(mut self, result: impl Into<Response>) -> io::Result<()> {
        self.send(Message::response(self.id, result.into())).await
    }

    pub async fn send(&mut self, message: Message) -> io::Result<()> {
        let _ = self.sender.send(message).await;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Receipt {
    sender: oneshot::Sender<bool>,
}

impl Receipt {
    pub fn new(accept: oneshot::Sender<bool>) -> Self {
        Self { sender: accept }
    }

    pub fn null() -> Self {
        let (sender, _) = oneshot::channel();

        Self { sender }
    }

    pub fn accept(self) {
        let _ = self.sender.send(true);
    }

    pub fn reject(self) {
        let _ = self.sender.send(false);
    }
}
