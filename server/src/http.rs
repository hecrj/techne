use crate::transport::{self, Action, Transport};

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::stream;
use futures::{SinkExt, Stream, StreamExt};
use http::StatusCode;
use http::header::{self, HeaderValue};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use http_body_util::{Full, StreamBody};
use hyper::body::{Bytes, Frame, Incoming};
use hyper::service::service_fn;
use hyper_util::rt;
use hyper_util::server::conn::auto;
use tokio::net;
use tokio::task;

use std::io;

pub struct Http {
    connections: mpsc::Receiver<io::Result<Action>>,
}

impl Http {
    pub async fn bind(address: impl net::ToSocketAddrs) -> io::Result<Self> {
        let listener = net::TcpListener::bind(address).await?;
        let (mut sender, receiver) = mpsc::channel(10);

        drop(task::spawn(async move {
            loop {
                let stream = match listener.accept().await {
                    Ok((stream, _address)) => rt::TokioIo::new(stream),
                    Err(error) => {
                        log::error!("{error}");
                        let _ = sender.send(Err(error)).await;

                        return;
                    }
                };

                let sender = sender.clone();

                drop(task::spawn(async move {
                    let service = service_fn(|request| serve(request, sender.clone()));

                    if let Err(error) = auto::Builder::new(rt::TokioExecutor::new())
                        .serve_connection_with_upgrades(stream, service)
                        .await
                    {
                        log::error!("{error}");
                    }
                }));
            }
        }));

        Ok(Self {
            connections: receiver,
        })
    }
}

impl Transport for Http {
    async fn accept(&mut self) -> io::Result<Action> {
        if let Some(result) = self.connections.next().await {
            result
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "http worker stopped running",
            ))
        }
    }
}

async fn serve(
    request: hyper::Request<Incoming>,
    mut actions: mpsc::Sender<io::Result<Action>>,
) -> Result<Response, hyper::Error> {
    Ok(match (request.method(), request.uri().path()) {
        (&http::Method::GET, "/") => {
            let (sender, result) = oneshot::channel();
            let _ = actions.send(Ok(Action::Subscribe(sender))).await;

            handle(result).await
        }
        (&http::Method::POST, "/") => {
            let bytes = request.into_body().collect().await?.to_bytes();

            let (sender, result) = oneshot::channel();
            let _ = actions.send(Ok(Action::Handle(bytes, sender))).await;

            handle(result).await
        }
        _ => not_found(),
    })
}

async fn handle(result: oneshot::Receiver<transport::Result>) -> Response {
    let Ok(result) = result.await else {
        return internal_error();
    };

    match result {
        transport::Result::Accept => status(StatusCode::ACCEPTED),
        transport::Result::Reject => bad_request(),
        transport::Result::Send(message) => ok(message),
        transport::Result::Stream(messages) => stream(messages),
        transport::Result::Unsupported => status(StatusCode::METHOD_NOT_ALLOWED),
    }
}

fn empty() -> Response {
    Response::new(
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed(),
    )
}

fn ok(bytes: Bytes) -> Response {
    let mut response = Response::new(Full::new(bytes).map_err(|never| match never {}).boxed());

    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    response
}

fn stream(stream: impl Stream<Item = Bytes> + Send + Sync + 'static) -> Response {
    let mut response = Response::new(BoxBody::new(StreamBody::new(
        stream
            .flat_map(|bytes| {
                stream::iter([
                    Bytes::from_static(b"data:"),
                    bytes,
                    Bytes::from_static(b"\n\n"),
                ])
                .map(Frame::data)
            })
            .map(Ok),
    )));

    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );

    response
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
