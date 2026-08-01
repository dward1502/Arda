pub mod browser;
pub mod channel;
pub mod external_sources;
pub mod protocol;
pub mod server;
pub mod tools;

pub use browser::*;
pub use channel::*;
pub use server::McpServer;
pub use tools::*;

pub async fn init_mcp_server() -> anyhow::Result<McpServer> {
    let server = McpServer::new();
    server.register_default_tools().await;
    Ok(server)
}
