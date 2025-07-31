use crate::mcp::server::{Message, Notification, Request, Response};
use crate::mcp::{ErrorKind, Id};

use bytes::Bytes;
use futures::SinkExt;
use futures::channel::mpsc;
use futures::channel::oneshot;

use std::io;

pub trait Transport {
    fn accept(&mut self) -> impl Future<Output = io::Result<Action>>;
}

pub enum Action {
    Subscribe(Channel),
    Handle(Bytes, Channel),
    Quit,
}

pub type Channel = oneshot::Sender<Result>;

pub enum Result {
    Accept,
    Reject,
    Send(Bytes),
    Stream(mpsc::Receiver<Bytes>),
    Unsupported,
}

#[derive(Debug)]
pub(crate) struct Connection {
    id: Id,
    state: State,
    total_requests: Id,
}

impl Connection {
    pub fn new(id: Id, channel: Channel) -> Self {
        Self {
            id,
            state: State::Idle(channel),
            total_requests: Id::default(),
        }
    }

    pub async fn request(&mut self, request: Request) -> io::Result<()> {
        let id = self.total_requests.increment();

        self.stream(Message::request(id, request)).await
    }

    pub async fn notify(&mut self, notification: Notification) -> io::Result<()> {
        self.stream(Message::notification(notification)).await
    }

    pub async fn error(self, error: ErrorKind) -> io::Result<()> {
        let id = self.id;
        self.send(Message::error(Some(id), error)).await
    }

    pub async fn finish(self, result: impl Into<Response>) -> io::Result<()> {
        let id = self.id;
        self.send(Message::response(id, result.into())).await
    }

    pub async fn stream(&mut self, message: Message) -> io::Result<()> {
        let bytes = message.serialize()?;

        match &mut self.state {
            State::Idle(_) => {
                let (mut stream, receiver) = mpsc::channel(10);
                let _ = stream.send(bytes).await;

                if let State::Idle(sender) =
                    std::mem::replace(&mut self.state, State::Streaming(stream))
                {
                    let _ = sender.send(Result::Stream(receiver));
                }
            }
            State::Streaming(sender) => {
                let _ = sender.send(bytes).await;
            }
        }

        Ok(())
    }

    pub async fn send(self, message: Message) -> io::Result<()> {
        let bytes = message.serialize()?;

        match self.state {
            State::Idle(sender) => {
                let _ = sender.send(Result::Send(bytes));
            }
            State::Streaming(mut sender) => {
                let _ = sender.send(bytes).await;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
enum State {
    Idle(Channel),
    Streaming(mpsc::Sender<Bytes>),
}

#[derive(Debug)]
pub(crate) struct Receipt {
    channel: Channel,
}

impl Receipt {
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }

    #[allow(dead_code)]
    pub fn accept(self) {
        let _ = self.channel.send(Result::Accept);
    }

    pub fn reject(self) {
        let _ = self.channel.send(Result::Reject);
    }
}
