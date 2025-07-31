use crate::mcp::Bytes;

use futures::channel::mpsc;
use futures::future::BoxFuture;

use std::io;

pub trait Transport {
    fn listen(&self) -> BoxFuture<'static, io::Result<Channel>>;

    fn send(&self, bytes: Bytes) -> BoxFuture<'static, io::Result<Channel>>;
}

pub type Channel = mpsc::Receiver<Bytes>;
