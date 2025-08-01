use crate::mcp;
use crate::mcp::server::{Notification, Request, Response};
use crate::mcp::{Bytes, ErrorKind, Id};
use crate::transport::{Channel, Result};

use futures::SinkExt;
use futures::channel::mpsc;

use std::io;

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

        #[allow(unreachable_code)]
        self.stream(mcp::Request::new(id, request).serialize()?)
            .await
    }

    pub async fn notify(&mut self, notification: Notification) -> io::Result<()> {
        #[allow(unreachable_code)]
        self.stream(mcp::Notification::new(notification).serialize()?)
            .await
    }

    pub async fn error(self, error: ErrorKind) -> io::Result<()> {
        let id = self.id;

        self.send(mcp::Error::new(Some(id), error).serialize()?)
            .await
    }

    pub async fn finish(self, result: impl Into<Response>) -> io::Result<()> {
        let id = self.id;

        self.send(mcp::Response::new(id, result.into()).serialize()?)
            .await
    }

    pub async fn stream(&mut self, bytes: Bytes) -> io::Result<()> {
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

    pub async fn send(self, bytes: Bytes) -> io::Result<()> {
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
