use crate::Message;
use crate::log;
use crate::server::transport::{Action, Connection, Receipt, Transport};

use futures::StreamExt;
use futures::channel::mpsc;
use serde::Serialize;
use tokio::io::{self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, Stdin};
use tokio::task;

pub struct Stdio<I = Stdin> {
    input: BufReader<I>,
    output: mpsc::Sender<Message>,
    json: String,
}

impl Stdio {
    pub fn current() -> Self {
        Stdio::custom(io::stdin(), io::stdout())
    }
}

impl<I> Stdio<I> {
    pub fn custom(input: I, mut output: impl AsyncWrite + Send + Unpin + 'static) -> Self
    where
        I: AsyncRead,
    {
        let (sender, mut receiver) = mpsc::channel(10);

        drop(task::spawn(async move {
            while let Some(message) = receiver.next().await {
                write(message, &mut output).await?;
            }

            Ok::<(), io::Error>(())
        }));

        Self {
            input: BufReader::new(input),
            output: sender,
            json: String::new(),
        }
    }
}

impl<I> Transport for Stdio<I>
where
    I: AsyncRead + Unpin,
{
    async fn accept(&mut self) -> io::Result<Action> {
        loop {
            let n = self.input.read_line(&mut self.json).await?;

            if n == 0 {
                return Ok(Action::Quit);
            }

            let message = serde_json::from_str(&self.json).inspect_err(log::error);
            self.json.clear();

            let Ok(message) = message else {
                continue;
            };

            let action = match message {
                Message::Request(request) => {
                    Action::Request(Connection::new(request.id, self.output.clone()), request)
                }
                Message::Notification(notification) => {
                    Action::Notify(Receipt::null(), notification)
                }
                Message::Response(response) => Action::Response(Receipt::null(), response),
                Message::Error(_) => continue,
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
