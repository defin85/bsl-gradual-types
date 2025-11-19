// Тест для диагностики create_session
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Простой тест: попытка создать сессию напрямую
    println!("Testing SessionManager::create_session...");

    let manager = mcp_debug_server::session::SessionManager::new();

    let binary_path = r"C:\1CProject\bsl-gradual-types-milestone-4.4\test_simple.exe".to_string();
    let adapter = "lldb".to_string();

    println!("Creating session for: {}", binary_path);
    println!("Using adapter: {}", adapter);

    match tokio::time::timeout(
        Duration::from_secs(10),
        manager.create_session(binary_path, adapter)
    ).await {
        Ok(Ok(session_id)) => {
            println!("✓ Session created: {}", session_id);
        }
        Ok(Err(e)) => {
            eprintln!("✗ Failed to create session: {}", e);
        }
        Err(_) => {
            eprintln!("✗ Timeout after 10 seconds");
        }
    }
}
