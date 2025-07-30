use crate::Message;
use crate::error;
use crate::notification;
use crate::request;
use crate::server::transport::{Action, Connection, Receipt, Transport};

use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::io::{self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::task;

pub struct Stdio {
    input: BufReader<Box<dyn AsyncRead + Send + Unpin>>,
    output: mpsc::Sender<Message>,
    json: String,
}

impl Stdio {
    pub fn current() -> Self {
        Stdio::custom(io::stdin(), io::stdout())
    }

    pub fn custom(
        input: impl AsyncRead + Send + Unpin + 'static,
        mut output: impl AsyncWrite + Send + Unpin + 'static,
    ) -> Self {
        let (sender, mut receiver) = mpsc::channel(10);

        drop(task::spawn(async move {
            while let Some(message) = receiver.next().await {
                write(message, &mut output).await?;
            }

            Ok::<(), io::Error>(())
        }));

        Self {
            input: BufReader::new(Box::new(input)),
            output: sender,
            json: String::new(),
        }
    }
}

impl Transport for Stdio {
    async fn accept(&mut self) -> io::Result<Action> {
        loop {
            self.json.clear();

            if self.input.read_line(&mut self.json).await? == 0 {
                return Ok(Action::Quit);
            }

            let message = match Message::deserialize(self.json.as_bytes()) {
                Ok(message) => message,
                Err(error) => {
                    log::error!("{error}");

                    let _ = self.output.send(Message::Error(error)).await;
                    continue;
                }
            };

            let action = match message {
                Message::Request(request::Message { id, request, .. }) => {
                    Action::Request(Connection::new(id, self.output.clone()), request)
                }
                Message::Notification(notification::Message { notification, .. }) => {
                    Action::Notify(Receipt::null(), notification)
                }
                Message::Response(response) => Action::Respond(Receipt::null(), response),
                Message::Error(error::Message { error, .. }) => {
                    log::error!("{error}");
                    continue;
                }
            };

            return Ok(action);
        }
    }
}

async fn write(
    data: impl Serialize + Send + Sync,
    writer: &mut (dyn AsyncWrite + Send + Unpin),
) -> io::Result<()> {
    let json = serde_json::to_vec(&data)?;

    writer.write_all(&json).await?;
    writer.write_u8(0xA).await?;
    writer.flush().await
}
