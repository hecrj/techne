use crate::server::transport::{self, Action, Delivery, Transport};
use crate::{Message, Notification, Request};

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future;
use futures::{SinkExt, Stream, StreamExt};
use http::header::{self, HeaderValue};
use http::{Method, StatusCode};
use http_body_util::StreamBody;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::Error;
use hyper::body::{Bytes, Frame, Incoming};
use hyper::service::service_fn;
use hyper_util::rt;
use hyper_util::server::conn::auto;
use serde::Serialize;
use tokio::net;
use tokio::task;

use std::io;

pub struct Http {
    listener: net::TcpListener,
}

impl Http {
    pub async fn bind(address: impl net::ToSocketAddrs) -> io::Result<Self> {
        let listener = net::TcpListener::bind(address).await?;

        Ok(Self { listener })
    }
}

impl Transport for Http {
    type Connection = Connection;
    type Decision = Decision;

    async fn connect(
        &mut self,
    ) -> io::Result<impl Stream<Item = Action<Self::Connection, Self::Decision>> + Send + 'static>
    {
        let (stream, _address) = self.listener.accept().await?;
        let stream = rt::TokioIo::new(stream);

        let (sender, receiver) = mpsc::channel(10);
        let service = service_fn(move |request| serve(request, sender.clone()));

        drop(task::spawn(async move {
            if let Err(error) = auto::Builder::new(rt::TokioExecutor::new())
                .serve_connection_with_upgrades(stream, service)
                .await
            {
                log::error!("{error}");
            }
        }));

        Ok(receiver)
    }
}

pub struct Connection {
    id: u64,
    status: Option<oneshot::Sender<StatusCode>>,
    body: mpsc::Sender<Bytes>,
}

impl transport::Connection for Connection {
    fn request<T: Serialize>(
        &mut self,
        request: Request<T>,
    ) -> impl Future<Output = io::Result<()>> + Send {
        if let Some(status) = self.status.take() {
            let _ = status.send(StatusCode::OK);
        }

        let bytes = serialize(&Message::Request(request));

        async {
            let _ = self.body.send(bytes).await;

            Ok(())
        }
    }

    fn notify<T: Serialize>(
        &mut self,
        notification: Notification<T>,
    ) -> impl Future<Output = io::Result<()>> + Send {
        if let Some(status) = self.status.take() {
            let _ = status.send(StatusCode::OK);
        }

        let bytes = serialize(&Message::Notification(notification));

        async {
            let _ = self.body.send(bytes).await;

            Ok(())
        }
    }

    fn finish<T: Serialize>(mut self, response: T) -> impl Future<Output = io::Result<()>> + Send {
        if let Some(status) = self.status.take() {
            let _ = status.send(StatusCode::OK);
        }

        let bytes = serialize(&Message::Response(crate::Response::new(self.id, response)));

        log::debug!("Finishing connection: {bytes:?}");

        async move {
            let _ = self.body.send(bytes).await;

            Ok(())
        }
    }

    fn reject(mut self) -> impl Future<Output = io::Result<()>> + Send {
        if let Some(status) = self.status.take() {
            let _ = status.send(StatusCode::BAD_REQUEST);
        }

        future::ready(Ok(()))
    }
}

pub struct Decision {
    status: oneshot::Sender<StatusCode>,
}

impl transport::Decision for Decision {
    fn accept(self) -> impl Future<Output = io::Result<()>> + Send {
        let _ = self.status.send(StatusCode::ACCEPTED);

        future::ready(Ok(()))
    }

    fn reject(self) -> impl Future<Output = io::Result<()>> + Send {
        let _ = self.status.send(StatusCode::BAD_REQUEST);

        future::ready(Ok(()))
    }
}

async fn serve(
    request: hyper::Request<Incoming>,
    mut sender: mpsc::Sender<Action<Connection, Decision>>,
) -> Result<hyper::Response<BoxBody<Bytes, Error>>, Error> {
    match request.uri().path() {
        "/" => {
            // TODO: Subscriptions (?)
            if request.method() == Method::GET {
                return Ok(status(StatusCode::METHOD_NOT_ALLOWED));
            }

            let (status_sender, status_receiver) = oneshot::channel();
            let (body_sender, body_receiver) = mpsc::channel(10);

            let bytes = request.into_body().collect().await?.to_bytes();

            let Ok(message): Result<Message, _> = serde_json::from_slice(&bytes) else {
                return Ok(bad_request());
            };

            let action = match message {
                Message::Request(request) => Action::Request(
                    Connection {
                        id: request.id,
                        status: Some(status_sender),
                        body: body_sender,
                    },
                    request,
                ),
                Message::Notification(notification) => Action::Deliver(
                    Decision {
                        status: status_sender,
                    },
                    Delivery::Notification(notification),
                ),
                Message::Response(response) => Action::Deliver(
                    Decision {
                        status: status_sender,
                    },
                    Delivery::Response(response),
                ),
            };

            if let Err(error) = sender.send(action).await {
                log::error!("{error}");
                return Ok(internal_error());
            }

            let Ok(status_code) = status_receiver.await else {
                return Ok(empty());
            };

            if !status_code.is_success() {
                return Ok(status(status_code));
            }

            Ok(stream(status_code, body_receiver))
        }
        _ => Ok(not_found()),
    }
}

fn empty() -> Response {
    Response::new(
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed(),
    )
}

fn stream(
    status: StatusCode,
    stream: impl Stream<Item = Bytes> + Send + Sync + 'static,
) -> Response {
    let mut response = Response::new(BoxBody::new(StreamBody::new(
        stream.map(Frame::data).map(Ok),
    )));

    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );

    response
}

fn serialize<T: Serialize>(message: &Message<T>) -> Bytes {
    let mut json: Vec<_> = "data: ".bytes().collect();
    serde_json::to_writer(&mut json, message).expect("Message serialization failed");
    json.extend("\n\n".bytes());

    Bytes::from(json)
}

fn bad_request() -> Response {
    status(StatusCode::BAD_REQUEST)
}

fn not_found() -> Response {
    status(StatusCode::NOT_FOUND)
}

fn internal_error() -> Response {
    status(StatusCode::INTERNAL_SERVER_ERROR)
}

fn status(code: StatusCode) -> Response {
    let mut response = empty();

    *response.status_mut() = code;

    response
}

type Response = hyper::Response<BoxBody<Bytes, hyper::Error>>;
