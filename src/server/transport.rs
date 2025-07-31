use bytes::Bytes;
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
