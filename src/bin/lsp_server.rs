//! LSP Server for BSL Gradual Type System

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("BSL LSP Server starting...");
    
    // TODO: Implement LSP server
    info!("LSP server not yet implemented");
    
    Ok(())
}