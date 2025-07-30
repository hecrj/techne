use crate::mcp;
use crate::mcp::client;
use crate::mcp::server::Message;
use crate::server::transport::{Action, Connection, Receipt, Transport};

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::{SinkExt, Stream, StreamExt};
use http::header::{self, HeaderValue};
use http::{Method, StatusCode};
use http_body_util::StreamBody;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
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
            let service = service_fn(|request| serve(request, sender.clone()));

            loop {
                let stream = match listener.accept().await {
                    Ok((stream, _address)) => rt::TokioIo::new(stream),
                    Err(error) => {
                        log::error!("{error}");
                        let _ = sender.send(Err(error)).await;

                        return;
                    }
                };

                if let Err(error) = auto::Builder::new(rt::TokioExecutor::new())
                    .serve_connection_with_upgrades(stream, service)
                    .await
                {
                    log::error!("{error}");
                }
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
    match request.uri().path() {
        "/" => {
            // TODO: Subscriptions (?)
            if request.method() == Method::GET {
                return Ok(status(StatusCode::METHOD_NOT_ALLOWED));
            }

            let bytes = request.into_body().collect().await?.to_bytes();
            let message = match client::Message::deserialize(&bytes) {
                Ok(message) => message,
                Err(error) => {
                    log::error!("{error}");

                    return Ok(protocol_error(None, error));
                }
            };

            let (sender, receiver) = mpsc::channel(10);
            let (accept_sender, accept_receiver) = oneshot::channel();
            let is_request = matches!(message, client::Message::Request(_));

            let action = match message {
                client::Message::Request(request) => {
                    let _ = accept_sender.send(true);
                    Action::Request(Connection::new(request.id, sender), request.payload)
                }
                client::Message::Notification(notification) => {
                    Action::Notify(Receipt::new(accept_sender), notification.payload)
                }
                client::Message::Response(response) => {
                    Action::Respond(Receipt::new(accept_sender), response)
                }
                client::Message::Error { id, error, .. } => {
                    return Ok(protocol_error(id, error));
                }
            };

            if let Err(error) = actions.send(Ok(action)).await {
                log::error!("{error}");
                return Ok(internal_error());
            }

            let Ok(true) = accept_receiver.await else {
                return Ok(bad_request());
            };

            Ok(stream(
                if is_request {
                    StatusCode::OK
                } else {
                    StatusCode::ACCEPTED
                },
                receiver,
            ))
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
    stream: impl Stream<Item = Message> + Send + Sync + 'static,
) -> Response {
    let mut response = Response::new(BoxBody::new(StreamBody::new(
        stream.map(serialize).map(Frame::data).map(Ok),
    )));

    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );

    response
}

fn serialize(message: Message) -> Bytes {
    log::debug!("Serializing: {message:?}");

    let mut json: Vec<_> = "data: ".bytes().collect();
    serde_json::to_writer(&mut json, &message).expect("Message serialization failed");
    json.extend("\n\n".bytes());

    Bytes::from(json)
}

fn bad_request() -> Response {
    status(StatusCode::BAD_REQUEST)
}

fn not_found() -> Response {
    status(StatusCode::NOT_FOUND)
}

fn protocol_error(id: Option<mcp::Id>, error: mcp::Error) -> Response {
    use futures::stream;

    stream(
        StatusCode::BAD_REQUEST,
        stream::once(async move { Message::error(id, error) }),
    )
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
