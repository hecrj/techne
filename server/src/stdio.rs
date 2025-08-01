use crate::mcp::Bytes;
use crate::transport::{Action, Result, Transport};

use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::{SinkExt, StreamExt};
use tokio::io::{self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::task;

pub struct Stdio {
    input: BufReader<Box<dyn AsyncRead + Send + Unpin>>,
    output: mpsc::Sender<Result>,
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
            while let Some(action) = receiver.next().await {
                match action {
                    Result::Send(bytes) => write(&bytes, &mut output).await?,
                    Result::Stream(mut stream) => {
                        while let Some(bytes) = stream.next().await {
                            write(&bytes, &mut output).await?
                        }
                    }
                    Result::Accept | Result::Reject | Result::Unsupported => {}
                }
            }

            Ok::<(), io::Error>(())
        }));

        Self {
            input: BufReader::new(Box::new(input)),
            output: sender,
        }
    }
}

impl Transport for Stdio {
    async fn accept(&mut self) -> io::Result<Action> {
        let mut line = Vec::new();

        if self.input.read_until(0xA, &mut line).await? == 0 {
            return Ok(Action::Quit);
        }

        let mut output = self.output.clone();
        let (sender, receiver) = oneshot::channel();

        task::spawn(async move {
            if let Ok(result) = receiver.await {
                let _ = output.send(result).await;
            }
        });

        Ok(Action::Handle(Bytes::from_owner(line), sender))
    }
}

async fn write(data: &[u8], writer: &mut (dyn AsyncWrite + Send + Unpin)) -> io::Result<()> {
    writer.write_all(data).await?;
    writer.write_u8(0xA).await?;
    writer.flush().await
}
