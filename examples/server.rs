use techne::Server;
use techne::server::Http;
use techne::tool::{string, tool, tool_2, u32};

use std::io;

#[tokio::main]
pub async fn main() -> io::Result<()> {
    let tools = [
        tool(
            say_hello,
            "Says hello to someone!",
            string("name", "The name to say hello to"),
        ),
        tool_2(
            add,
            "Adds two integers",
            u32("a", "The first operand"),
            u32("b", "The second operand"),
        ),
    ];

    let server = Server::new().tools(tools);
    let transport = Http::bind("127.0.0.1:8080").await?;

    server.run(transport).await
}

async fn say_hello(name: String) -> String {
    format!("Hello, {name}!")
}

async fn add(a: u32, b: u32) -> u32 {
    a + b
}
