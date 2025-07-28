use techne::Client;
use techne::client::Stdio;

use std::io;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    let transport = Stdio::run("cargo", ["run", "--example", "server"])?;

    let mut client = Client::new(
        "techne-client-example",
        env!("CARGO_PKG_VERSION"),
        transport,
    )
    .await?;

    dbg!(&client);
    dbg!(client.list_tools().await?);

    Ok(())
}
