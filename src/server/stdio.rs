use crate::server;
use crate::{Message, Notification, Request, Response};

use futures::stream::{self, Stream};
use serde::Serialize;
use tokio::io::{
    self, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, Stdin, Stdout,
};
use tokio::sync::Mutex;

use std::ops::DerefMut;
use std::sync::Arc;

pub struct Stdio<I = Stdin, O = Stdout> {
    input: BufReader<I>,
    output: Arc<Mutex<O>>,
    json: String,
}

impl Stdio {
    pub fn current() -> Self {
        Stdio::custom(io::stdin(), io::stdout())
    }
}

impl<I, O> Stdio<I, O> {
    pub fn custom(input: I, output: O) -> Self
    where
        I: AsyncRead,
    {
        Self {
            input: BufReader::new(input),
            output: Arc::new(Mutex::new(output)),
            json: String::new(),
        }
    }
}

impl<I, O> server::Transport for Stdio<I, O>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Send + Unpin + 'static,
{
    type Connection = Connection;
    type Decision = Decision;

    async fn connect(
        &mut self,
    ) -> io::Result<
        impl Stream<Item = server::Action<Self::Connection, Self::Decision>> + Send + 'static,
    > {
        let _ = self.input.read_line(&mut self.json).await?;
        let message = serde_json::from_str(&self.json);
        self.json.clear();

        let action = match message? {
            Message::Request(request) => server::Action::Request(
                Connection {
                    id: request.id,
                    output: self.output.clone(),
                },
                request,
            ),
            Message::Notification(notification) => {
                server::Action::Deliver(Decision, server::Delivery::Notification(notification))
            }
            Message::Response(response) => {
                server::Action::Deliver(Decision, server::Delivery::Response(response))
            }
        };

        Ok(stream::once(async move { action }))
    }
}

pub struct Connection {
    id: u64,
    output: Arc<Mutex<dyn AsyncWrite + Send + Unpin>>,
}

impl server::Connection for Connection {
    async fn request<T: Serialize + Send + Sync>(&mut self, request: Request<T>) -> io::Result<()> {
        write(request, self.output.lock().await.deref_mut()).await
    }

    async fn notify<T: Serialize + Send + Sync>(
        &mut self,
        notification: Notification<T>,
    ) -> io::Result<()> {
        write(notification, self.output.lock().await.deref_mut()).await
    }

    async fn finish<T: Serialize + Send + Sync>(self, response: T) -> io::Result<()> {
        write(
            &Response::new(self.id, response),
            self.output.lock().await.deref_mut(),
        )
        .await
    }

    async fn reject(self) -> io::Result<()> {
        Ok(())
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

pub struct Decision;

impl server::Decision for Decision {
    async fn accept(self) -> io::Result<()> {
        Ok(())
    }

    async fn reject(self) -> io::Result<()> {
        Ok(())
    }
}
