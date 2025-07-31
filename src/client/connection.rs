use crate::client::transport::Channel;
use crate::mcp;
use crate::mcp::server;

use futures::StreamExt;
use serde::de::DeserializeOwned;

use std::io;

pub struct Connection {
    channel: Channel,
}

impl Connection {
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }

    pub(crate) async fn next<T: DeserializeOwned>(&mut self) -> io::Result<server::Message<T>> {
        let Some(bytes) = self.channel.next().await else {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "stream was closed by peer",
            ));
        };

        match server::Message::deserialize(&bytes) {
            Ok(message) => Ok(message),
            Err(error) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid message: {error}"),
            )),
        }
    }

    pub(crate) async fn response<T: DeserializeOwned>(mut self) -> io::Result<mcp::Response<T>> {
        loop {
            if let server::Message::Response(response) = self.next().await? {
                return Ok(response);
            }
        }
    }
}
