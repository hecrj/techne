use techne::Client;
use techne::client::{Http, Stdio};

use serde_json::json;
use std::env;
use std::io;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let version = env!("CARGO_PKG_VERSION");
    let use_http = env::args().nth(1).as_deref() == Some("--http");

    let mut client = if use_http {
        // Run `cargo run --example client -- --http` first!
        let transport = Http::new("http://127.0.0.1:8080")?;

        Client::new("techne-http-client-example", version, transport).await?
    } else {
        let transport = Stdio::run("cargo", ["run", "--example", "server"])?;

        Client::new("techne-stdio-client-example", version, transport).await?
    };

    let tools = client.list_tools().await?;

    let hello = client
        .call_tool("say_hello", json!({ "name": "World" }))
        .await?;

    dbg!(tools);
    dbg!(hello);

    Ok(())
}
