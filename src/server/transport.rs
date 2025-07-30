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
    total_requests: u64,
}

impl Connection {
    pub fn new(id: u64, sender: mpsc::Sender<Message>) -> Self {
        Self {
            id,
            sender,
            total_requests: 0,
        }
    }

    pub async fn request(&mut self, request: Request) -> io::Result<()> {
        let id = self.total_requests;
        self.total_requests += 1;

        self.send(Message::Request(request.stamp(id))).await
    }

    pub async fn notify(&mut self, notification: Notification) -> io::Result<()> {
        self.send(Message::Notification(notification.stamp())).await
    }

    pub async fn error(mut self, error: Error) -> io::Result<()> {
        self.send(Message::Error(error.stamp(Some(self.id)))).await
    }

    pub async fn finish<T: Serialize>(mut self, result: T) -> io::Result<()> {
        self.send(Message::Response(
            Response {
                jsonrpc: crate::JSONRPC.to_owned(),
                id: self.id,
                result,
            }
            .serialize()?,
        ))
        .await
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
