use crate::mcp::Bytes;
use crate::transport::{Channel, Transport};

use futures::SinkExt;
use futures::channel::mpsc;
use futures::future::{BoxFuture, FutureExt};
use reqwest::header;
use reqwest::{Client, Error, IntoUrl, Response, Url};
use tokio::task;

use std::io;

pub struct Http {
    client: Client,
    address: Url,
}

impl Http {
    pub fn new(address: impl IntoUrl) -> io::Result<Self> {
        Ok(Self {
            client: Client::new(),
            address: address.into_url().map_err(to_error)?,
        })
    }
}

impl Transport for Http {
    fn listen(&self) -> BoxFuture<'static, io::Result<Channel>> {
        let client = self.client.clone();
        let address = self.address.clone();

        async move {
            let response = client
                .get(address)
                .header(header::ACCEPT, "text/event-stream")
                .send()
                .await
                .map_err(to_error)?
                .error_for_status()
                .map_err(to_error)?;

            let (sender, receiver) = mpsc::channel(10);

            drop(task::spawn(async move {
                if let Err(error) = read_stream(sender, response).await {
                    log::error!("{error}");
                }
            }));

            Ok(receiver)
        }
        .boxed()
    }

    fn send(&self, bytes: Bytes) -> BoxFuture<'static, io::Result<Channel>> {
        let client = self.client.clone();
        let address = self.address.clone();

        async move {
            let response = client
                .post(address)
                .header(header::ACCEPT, "application/json, text/event-stream")
                .body(bytes)
                .send()
                .await
                .map_err(to_error)?
                .error_for_status()
                .map_err(to_error)?;

            match response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|header| header.to_str().ok())
            {
                Some("application/json") => {
                    let (mut sender, receiver) = mpsc::channel(1);

                    let bytes = response.bytes().await.map_err(to_error)?;
                    let _ = sender.send(bytes).await;

                    Ok(receiver)
                }
                Some("text/event-stream") => {
                    let (sender, receiver) = mpsc::channel(10);

                    drop(task::spawn(async move {
                        if let Err(error) = read_stream(sender, response).await {
                            log::error!("{error}");
                        }
                    }));

                    Ok(receiver)
                }
                content_type => Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!("invalid server content-type: {content_type:?}"),
                )),
            }
        }
        .boxed()
    }
}

async fn read_stream(mut sender: mpsc::Sender<Bytes>, mut response: Response) -> Result<(), Error> {
    static PREFIX: usize = b"data:".len();

    let mut last_event = Vec::new();

    while let Some(chunk) = response.chunk().await? {
        for chunk in chunk.split(|byte| *byte == 0xA) {
            if chunk.is_empty() && !last_event.is_empty() {
                if last_event.len() > PREFIX {
                    let _ = sender
                        .send(Bytes::copy_from_slice(&last_event[PREFIX..]))
                        .await;
                }

                last_event.clear();
            }

            last_event.extend_from_slice(chunk);
        }
    }

    Ok(())
}

fn to_error(error: Error) -> io::Error {
    if error.is_builder() {
        return io::Error::new(io::ErrorKind::InvalidInput, error.to_string());
    }

    if error.is_connect() {
        return io::Error::new(io::ErrorKind::ConnectionRefused, error.to_string());
    }

    io::Error::other(error.to_string())
}
