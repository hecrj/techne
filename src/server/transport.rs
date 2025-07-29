use crate::error;
use crate::{Error, Message, Notification, Request, Response};

use futures::SinkExt;
use futures::channel::mpsc;
use futures::channel::oneshot;
use serde::Serialize;

use std::io;

pub trait Transport {
    fn accept(&mut self) -> impl Future<Output = io::Result<Action>>;
}

pub enum Action {
    Request(Connection, Request),
    Notify(Receipt, Notification),
    Respond(Receipt, Response),
    Quit,
}

#[derive(Debug, Clone)]
pub struct Connection {
    id: u64,
    sender: mpsc::Sender<Message>,
}

impl Connection {
    pub fn new(id: u64, sender: mpsc::Sender<Message>) -> Self {
        Self { id, sender }
    }

    pub async fn request<T: Serialize>(&mut self, request: Request<T>) -> io::Result<()> {
        self.send(Message::Request(request)).await
    }

    pub async fn notify<T: Serialize>(&mut self, notification: Notification<T>) -> io::Result<()> {
        self.send(Message::Notification(notification)).await
    }

    pub async fn error<T: Serialize>(mut self, error: error::Body<T>) -> io::Result<()> {
        self.send(Message::Error(Error {
            jsonrpc: crate::JSONRPC.to_owned(),
            id: self.id,
            error,
        }))
        .await
    }

    pub async fn finish<T: Serialize>(mut self, result: T) -> io::Result<()> {
        self.send(Message::Response(Response {
            jsonrpc: crate::JSONRPC.to_owned(),
            id: self.id,
            result,
        }))
        .await
    }

    pub async fn send<T: Serialize>(&mut self, message: Message<T>) -> io::Result<()> {
        let _ = self.sender.send(message.serialize()?).await;

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
