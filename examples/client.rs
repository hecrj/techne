use techne::Client;
use techne::client::Stdio;

use serde_json::json;
use std::io;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let transport = Stdio::run("cargo", ["run", "--example", "server"])?;

    let mut client = Client::new(
        "techne-client-example",
        env!("CARGO_PKG_VERSION"),
        transport,
    )
    .await?;

    let tools = client.list_tools().await?;

    let hello = client
        .call_tool("say_hello", json!({ "name": "World" }))
        .await?;

    dbg!(tools);
    dbg!(hello);

    Ok(())
}
