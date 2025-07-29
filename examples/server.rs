use techne::server::{self, Server};
use techne::tool::{string, tool, tool_2, u32};

use std::env;
use std::io;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let server = Server::new("techne-server-example", env!("CARGO_PKG_VERSION"));
    let transport = server::transport(env::args()).await?;

    let tools = [
        tool(say_hello, string("name", "The name to say hello to"))
            .name("say_hello")
            .description("Say hello to someone"),
        tool_2(
            add,
            u32("a", "The first operand"),
            u32("b", "The second operand"),
        )
        .name("add")
        .description("Adds two integers"),
    ];

    server.tools(tools).run(transport).await
}

async fn say_hello(name: String) -> String {
    format!("Hello, {name}!")
}

async fn add(a: u32, b: u32) -> u32 {
    a + b
}
