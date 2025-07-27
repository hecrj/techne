<div align="center">

# Techne

> A state involving true reason concerned with production.
>
> — Aristotle

[![Documentation](https://docs.rs/techne/badge.svg)](https://docs.rs/techne)
[![Crates.io](https://img.shields.io/crates/v/techne.svg)](https://crates.io/crates/techne)
[![License](https://img.shields.io/crates/l/techne.svg)](https://github.com/hecrj/techne/blob/master/LICENSE)
[![Downloads](https://img.shields.io/crates/d/techne.svg)](https://crates.io/crates/techne)
[![Test Status](https://img.shields.io/github/actions/workflow/status/hecrj/techne/test.yml?branch=master&event=push&label=test)](https://github.com/hecrj/techne/actions)

An MCP implementation for Rust focused on simplicity and type-safety.
</div>

## Features

- No macros!
- Coherent schemas enforced at the type level
- Custom transports
- Latest protocol version (`2025-06-18`)

## Server
List any Rust functions you want to expose as tools, then create a `Server` and
run it with your desired transport:

```rust
use techne::Server;
use techne::server::Http;
use techne::tool::{tool, string};

use std::io;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    let tools = [
        tool(say_hello, string("name", "The name to say hello to"))
            .name("say_hello")
            .description("Say hello to someone"),
    ];

    let server = Server::new().tools(tools);
    let transport = Http::bind("127.0.0.1:8080").await?;

    server.run(transport).await
}

async fn say_hello(name: String) -> String {
    format!("Hello, {name}!")
}
```
