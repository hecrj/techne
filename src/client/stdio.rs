use crate::Message;
use crate::client::transport::{Receiver, Task, Transport};

use futures::channel::mpsc;
use futures::future;
use futures::{FutureExt, SinkExt, StreamExt};
use tokio::io::{self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process;
use tokio::task;

use std::ffi::OsStr;

pub struct Stdio {
    _process: process::Child,
    runner: mpsc::Sender<Action>,
}

type Sender = mpsc::Sender<io::Result<Message>>;

enum Action {
    Listen(Sender),
    Send(Message, Sender),
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
    fn listen(&self) -> Task {
        let mut runner = self.runner.clone();

        async move {
            let (sender, receiver) = mpsc::channel(1);
            let _ = runner.send(Action::Listen(sender)).await;

            Ok(Receiver::new(receiver))
        }
        .boxed()
    }

    fn send(&self, message: Message) -> Task {
        let mut runner = self.runner.clone();

        async move {
            let (sender, receiver) = mpsc::channel(1);
            let _ = runner.send(Action::Send(message, sender)).await;

            Ok(Receiver::new(receiver))
        }
        .boxed()
    }
}

async fn run(
    mut input: impl AsyncWrite + Unpin,
    output: impl AsyncRead + Unpin,
    mut receiver: mpsc::Receiver<Action>,
) -> io::Result<()> {
    use future::Either;

    let mut output = BufReader::new(output).lines();
    let mut listeners = Vec::new();

    loop {
        let next_event = {
            let next_line = Box::pin(output.next_line());
            let next_action = receiver.select_next_some().fuse();

            future::select(next_line, next_action)
        };

        let event = match next_event.await {
            Either::Left((line, _)) => Either::Left(line),
            Either::Right((action, _)) => Either::Right(action),
        };

        match event {
            Either::Right(Action::Listen(sender)) => {
                listeners.push(sender);
            }
            Either::Left(Ok(Some(line))) => {
                let Ok(message) = deserialize(&line).await else {
                    continue;
                };

                for listener in &mut listeners {
                    let _ = listener.send(Ok(message.clone())).await;
                }
            }
            Either::Right(Action::Send(message, mut sender)) => match message {
                Message::Request(request) => {
                    let request_id = request.id;
                    write(&mut input, &Message::Request(request)).await?;

                    loop {
                        let Some(line) = output.next_line().await? else {
                            return Ok(());
                        };

                        let message = deserialize(&line).await;

                        match message {
                            Ok(Message::Response(response)) if response.id == request_id => {
                                let _ = sender.send(Ok(Message::Response(response))).await;
                                break;
                            }
                            _ => {
                                let _ = sender.send(message).await;
                            }
                        }
                    }
                }
                Message::Notification(_) | Message::Response(_) | Message::Error(_) => {
                    write(&mut input, &message).await?;
                }
            },
            _ => {
                break;
            }
        }
    }

    Ok(())
}

async fn deserialize(json: &str) -> io::Result<Message> {
    // TODO: Deserialize in blocking task (?)
    Ok(serde_json::from_str(json)?)
}

async fn write(input: &mut (impl AsyncWrite + Unpin), message: &Message) -> io::Result<()> {
    // TODO: Serialize in blocking task (?)
    let json = serde_json::to_vec(message)?;

    input.write_all(&json).await?;
    input.write_u8(0xA).await?;
    input.flush().await
}
