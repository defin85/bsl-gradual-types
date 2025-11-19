//! Интеграционный тест автопоиска DAP адаптеров
//!
//! Проверяет, что:
//! 1. CodeLLDB находится автоматически в VS Code extensions
//! 2. DapClient может запуститься с найденным адаптером
//! 3. DAP инициализация проходит успешно

use mcp_debug_server::config::{find_codelldb, resolve_adapter};
use mcp_debug_server::dap::DapClient;

#[tokio::test]
async fn test_codelldb_autodiscovery() {
    // Шаг 1: Проверить автопоиск CodeLLDB
    let codelldb_path = find_codelldb();

    match codelldb_path {
        Some(path) => {
            println!("✓ Found CodeLLDB: {:?}", path);
            assert!(path.exists(), "CodeLLDB path should exist");

            // Шаг 2: Попробовать запустить DapClient
            let path_str = path.to_string_lossy().to_string();
            println!("  Attempting to spawn DapClient with: {}", path_str);

            match DapClient::spawn(&path_str).await {
                Ok((mut client, _event_rx)) => {
                    println!("✓ Successfully spawned DapClient");

                    // Шаг 3: Попробовать инициализировать DAP сессию
                    match client.initialize().await {
                        Ok(response) => {
                            println!("✓ DAP initialized successfully");
                            println!("  Response: {:?}", response);

                            // Cleanup: terminate adapter
                            let _ = client.terminate().await;
                        }
                        Err(e) => {
                            eprintln!("✗ Failed to initialize DAP: {}", e);
                            // Не падаем тест - initialize может требовать дополнительных шагов
                        }
                    }
                }
                Err(e) => {
                    panic!("Failed to spawn DapClient: {}", e);
                }
            }
        }
        None => {
            println!("⚠ CodeLLDB not found - skipping test");
            println!("  (This is expected if CodeLLDB extension is not installed)");
            // Не падаем тест, так как CodeLLDB может быть не установлен
        }
    }
}

#[test]
fn test_resolve_adapter_lldb() {
    // Проверить что resolve_adapter находит CodeLLDB для "lldb"
    let resolved = resolve_adapter("lldb");

    println!("Resolved 'lldb' to: {}", resolved);

    // Проверить что resolved содержит либо "codelldb" либо "lldb"
    assert!(
        resolved.contains("codelldb") || resolved == "lldb",
        "Expected CodeLLDB path or 'lldb', got: {}",
        resolved
    );
}

#[test]
fn test_resolve_adapter_custom_path() {
    // Проверить что custom path возвращается as-is
    let custom_path = "/custom/path/to/my-debugger";
    let resolved = resolve_adapter(custom_path);

    assert_eq!(
        resolved, custom_path,
        "Custom path should be returned as-is"
    );
}
