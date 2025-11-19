//! Test lldb-dap compatibility with MCP Debug Server
//!
//! This example tests if official LLVM lldb-dap adapter works with our DAP client.
//!
//! # Prerequisites
//!
//! - LLVM 21.1.0+ installed (https://github.com/llvm/llvm-project/releases)
//! - lldb-dap.exe in PATH or at: C:\Program Files\LLVM\bin\lldb-dap.exe
//!
//! # Usage
//!
//! ```bash
//! cargo run --example test_lldb_dap
//! ```

use mcp_debug_server::dap::DapClient;
use mcp_debug_server::types::DapResult;

#[tokio::main]
async fn main() -> DapResult<()> {
    // Настроить tracing для debug вывода
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🧪 Testing lldb-dap compatibility...\n");

    // Попробовать найти lldb-dap
    let adapter_path = find_lldb_dap();
    println!("📍 Using adapter: {}\n", adapter_path);

    // Создать DAP client
    println!("🚀 Spawning lldb-dap process...");
    let (mut client, mut event_rx) = DapClient::spawn(&adapter_path).await?;
    println!("✅ lldb-dap spawned successfully!\n");

    // Запустить event handler в background
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            println!("📨 Received event: {}", event);
        }
    });

    // Инициализировать сессию
    println!("🔄 Sending initialize request...");
    let init_response = client.initialize().await?;
    println!("✅ Initialize response: {}\n", init_response);

    println!("🎉 SUCCESS: lldb-dap is compatible with MCP Debug Server!\n");
    println!("📝 You can use it by specifying adapter_type:");
    println!("   - Full path: \"C:\\\\Program Files\\\\LLVM\\\\bin\\\\lldb-dap.exe\"");
    println!("   - Short name (if in PATH): \"lldb-dap\"");

    Ok(())
}

/// Найти lldb-dap в стандартных локациях
fn find_lldb_dap() -> String {
    // Проверить стандартную установку LLVM на Windows
    let standard_path = r"C:\Program Files\LLVM\bin\lldb-dap.exe";
    if std::path::Path::new(standard_path).exists() {
        return standard_path.to_string();
    }

    // Fallback: попробовать найти в PATH
    "lldb-dap".to_string()
}
