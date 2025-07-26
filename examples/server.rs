use techne::Server;
use techne::server::Http;

use std::io;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    let server = Server::new();
    let transport = Http::bind("127.0.0.1:8080").await?;

    server.run(transport).await
}
