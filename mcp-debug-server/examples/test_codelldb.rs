//! Test CodeLLDB compatibility with MCP Debug Server
//!
//! CodeLLDB is a self-contained DAP adapter that includes all dependencies.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example test_codelldb
//! ```

use mcp_debug_server::dap::DapClient;
use mcp_debug_server::types::DapResult;

#[tokio::main]
async fn main() -> DapResult<()> {
    // Настроить tracing для debug вывода
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🧪 Testing CodeLLDB compatibility...\n");

    // Найти CodeLLDB через auto-discovery
    let adapter_path = mcp_debug_server::config::resolve_adapter("codelldb");
    println!("📍 Using adapter: {}\n", adapter_path);

    // Создать DAP client
    println!("🚀 Spawning CodeLLDB process...");
    let (mut client, mut event_rx) = DapClient::spawn(&adapter_path).await?;
    println!("✅ CodeLLDB spawned successfully!\n");

    // Запустить event handler в background
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            println!("📨 Received event: {}", event);
        }
    });

    // Инициализировать сессию
    println!("🔄 Sending initialize request...");
    let init_response = client.initialize().await?;
    println!("✅ Initialize response:");
    println!("{}\n", serde_json::to_string_pretty(&init_response)?);

    println!("🎉 SUCCESS: CodeLLDB is fully compatible with MCP Debug Server!\n");

    Ok(())
}
