use crate::{Notification, Request};

use futures::Stream;
use serde::Serialize;

use std::io;

pub trait Transport {
    type Connection: Connection + Send + 'static;
    type Decision: Decision + Send + 'static;

    fn connect(
        &mut self,
    ) -> impl Future<
        Output = io::Result<
            impl Stream<Item = Action<Self::Connection, Self::Decision>> + Send + 'static,
        >,
    >;
}

pub trait Connection {
    fn request<T: Serialize + Send + Sync>(
        &mut self,
        message: Request<T>,
    ) -> impl Future<Output = io::Result<()>> + Send;

    fn notify<T: Serialize + Send + Sync>(
        &mut self,
        message: Notification<T>,
    ) -> impl Future<Output = io::Result<()>> + Send;

    fn finish<T: Serialize + Send + Sync>(
        self,
        response: T,
    ) -> impl Future<Output = io::Result<()>> + Send;

    fn reject(self) -> impl Future<Output = io::Result<()>> + Send;
}

pub trait Decision {
    fn accept(self) -> impl Future<Output = io::Result<()>> + Send;

    fn reject(self) -> impl Future<Output = io::Result<()>> + Send;
}

#[derive(Debug)]
pub enum Action<C, D> {
    Request(C, Request),
    Deliver(D, Delivery),
}

#[derive(Debug)]
pub enum Delivery {
    Notification(Notification),
    Response(crate::Response),
}
