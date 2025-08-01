pub use techne_mcp as mcp;

#[cfg(feature = "client")]
pub use techne_client as client;

#[cfg(feature = "client")]
pub use client::Client;

#[cfg(feature = "server")]
pub use techne_server as server;

#[cfg(feature = "server")]
pub use server::Server;
