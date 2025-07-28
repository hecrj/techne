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

    pub(crate) async fn response<T: DeserializeOwned>(&mut self) -> io::Result<Response<T>> {
        let mut responses = Box::pin(self.raw.by_ref().filter_map(async |message| {
            if let Message::Response(response) = message.ok()? {
                Some(response)
            } else {
                None
            }
        }));

        let Some(response) = responses.next().await else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected response",
            ));
        };

        Ok(Response {
            jsonrpc: crate::JSONRPC.to_owned(),
            id: response.id,
            result: serde_json::from_value(response.result)?,
        })
    }
}
