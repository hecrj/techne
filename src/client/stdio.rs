use crate::client::transport::{Channel, Transport};
use crate::mcp::client::Message;
use crate::mcp::server;
use crate::mcp::{self, Bytes};

use futures::channel::mpsc;
use futures::future::{self, BoxFuture};
use futures::{FutureExt, SinkExt, StreamExt};
use tokio::io::{self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process;
use tokio::task;

use std::ffi::OsStr;

pub struct Stdio {
    _process: process::Child,
    runner: mpsc::Sender<Action>,
}

impl Stdio {
    pub fn run(
        command: impl AsRef<OsStr>,
        args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    ) -> io::Result<Self> {
        let mut process = process::Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null()) // TODO
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true) // TODO: Graceful quitting
            .spawn()?;

        let input = process.stdin.take().expect("process must have stdin");
        let output = process.stdout.take().expect("process must have stdout");

        let (sender, receiver) = mpsc::channel(10);
        drop(task::spawn(run(input, output, receiver)));

        Ok(Self {
            _process: process,
            runner: sender,
        })
    }
}

impl Transport for Stdio {
    fn listen(&self) -> BoxFuture<'static, io::Result<Channel>> {
        let mut runner = self.runner.clone();

        async move {
            let (sender, receiver) = mpsc::channel(1);
            let _ = runner.send(Action::Listen(sender)).await;

            Ok(receiver)
        }
        .boxed()
    }

    fn send(&self, bytes: Bytes) -> BoxFuture<'static, io::Result<Channel>> {
        let mut runner = self.runner.clone();

        async move {
            let (sender, receiver) = mpsc::channel(1);
            let _ = runner.send(Action::Send(bytes, sender)).await;

            Ok(receiver)
        }
        .boxed()
    }
}

type Sender = mpsc::Sender<Bytes>;

enum Action {
    Listen(Sender),
    Send(Bytes, Sender),
}

async fn run(
    mut input: impl AsyncWrite + Unpin,
    output: impl AsyncRead + Unpin,
    mut receiver: mpsc::Receiver<Action>,
) -> io::Result<()> {
    use future::Either;

    let mut output = BufReader::new(output);
    let mut listeners = Vec::new();
    let mut buffer = Vec::new();

    loop {
        let event = {
            let next_line = Box::pin(output.read_until(0xA, &mut buffer));
            let next_action = receiver.next().fuse();
            let next_event = future::select(next_line, next_action);

            match next_event.await {
                Either::Left((line, _)) => Either::Left(line),
                Either::Right((Some(action), _)) => Either::Right(action),
                _ => return Ok(()),
            }
        };

        match event {
            Either::Right(Action::Listen(sender)) => {
                listeners.push(sender);
            }
            Either::Left(Ok(n)) => {
                if n == 0 {
                    return Ok(());
                }

                let bytes = Bytes::from_owner(std::mem::take(&mut buffer));

                for listener in &mut listeners {
                    let _ = listener.send(bytes.clone()).await;
                }
            }
            Either::Right(Action::Send(bytes, mut sender)) => {
                write(&mut input, &bytes).await?;

                let Ok(Message::Request(_)) = Message::<mcp::Ignored>::deserialize(&bytes) else {
                    continue;
                };

                while let Ok(n) = output.read_until(0xA, &mut buffer).await {
                    if n == 0 {
                        return Ok(());
                    }

                    let bytes = Bytes::from_owner(std::mem::take(&mut buffer));
                    let _ = sender.send(bytes.clone()).await;

                    if let Ok(server::Message::Response(_)) =
                        server::Message::<mcp::Ignored>::deserialize(&bytes)
                    {
                        break;
                    }
                }
            }
            _ => {
                break;
            }
        }
    }

    Ok(())
}

async fn write(input: &mut (impl AsyncWrite + Unpin), data: &[u8]) -> io::Result<()> {
    input.write_all(data).await?;
    input.write_u8(0xA).await?;
    input.flush().await
}
