<div align="center">

# Techne

[![Documentation](https://docs.rs/techne/badge.svg)](https://docs.rs/techne)
[![Crates.io](https://img.shields.io/crates/v/techne.svg)](https://crates.io/crates/techne)
[![License](https://img.shields.io/crates/l/techne.svg)](https://github.com/hecrj/techne/blob/master/LICENSE)
[![Downloads](https://img.shields.io/crates/d/techne.svg)](https://crates.io/crates/techne)
[![Test Status](https://img.shields.io/github/actions/workflow/status/hecrj/techne/test.yml?branch=master&event=push&label=test)](https://github.com/hecrj/techne/actions)

An MCP implementation for Rust focused on simplicity and type-safety.
</div>

## Features

- Completely handmade!
- No macros!
- Coherent schemas enforced at the type level
- Stdio and Streamable HTTP transports
- Custom transports
- Latest protocol version (`2025-06-18`)

**Very experimental! Only the `tools` capability is currently supported.**

## Server
Create a `Server`, choose your desired transport, list your tools, and run:

```rust
use techne::Server;
use techne::server::Stdio;
use techne::server::tool::{tool, string};

use std::io;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    let server = Server::new("techne-server-example", env!("CARGO_PKG_VERSION"));
    let transport = Stdio::current();

    let tools = [
        tool(say_hello, string("name", "The name to say hello to"))
            .name("say_hello")
            .description("Say hello to someone"),
    ];

    server.tools(tools).run(transport).await
}

async fn say_hello(name: String) -> String {
    format!("Hello, {name}!")
}
```

## Client
Create a `Client` with your desired transport and query the server:

```rust
use techne::Client;
use techne::client::Stdio;
use techne::mcp::json;

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

    let tools = client.list_tools().await?;

    let hello = client
        .call_tool("say_hello", json!({ "name": "World" }))
        .await?;

    dbg!(tools);
    dbg!(hello);

    Ok(())
}
```
