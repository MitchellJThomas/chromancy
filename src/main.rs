// src/main.rs
use rmcp::{Server, ServerTransport};
use tokio::io::{stdin, stdout};

mod server;
mod tools;
mod wled_client;
mod types;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let server = Server::new()
        .with_tool(tools::GetDeviceInfo)
        .with_tool(tools::GetState)
        .with_tool(tools::SetPower)
        .with_tool(tools::SetBrightness)
        .with_tool(tools::SetColor)
        .with_tool(tools::SetEffect)
        .with_tool(tools::SetPalette)
        .with_tool(tools::SetSegment)
        .with_tool(tools::ListEffects)
        .with_tool(tools::ListPalettes);

    let transport = ServerTransport::stdio(stdin(), stdout());
    
    server.serve(transport).await?;
    
    Ok(())
}