mod dap;
mod server;
mod session;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};
use server::DebugServer;
use session::SessionManager;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Enable logs for easier debugging inside AI agents or CLI.
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let session_manager = SessionManager::default();
    let service = DebugServer::new(session_manager);

    tracing::info!("🚀 Starting MCP Debug Server (CodeLLDB bridge)");

    let server = service.serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}
