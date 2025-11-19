//! Manual debug test с реальным CodeLLDB
//!
//! Тестирует полный цикл отладки напрямую через SessionManager

use mcp_debug_server::dap::EventBuffer;
use mcp_debug_server::session::SessionManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Настроить tracing для debug вывода
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🧪 Manual Debug Test с CodeLLDB\n");

    // 1. Создать SessionManager и EventBuffer
    let manager = SessionManager::new();
    let event_buffer = EventBuffer::new();

    // 2. Создать debug сессию
    println!("📍 Creating debug session...");
    let session_id = manager
        .create_session(
            r"C:\1CProject\bsl-gradual-types-milestone-4.4\target\debug\examples\test_program.exe"
                .to_string(),
            "codelldb".to_string(),
            event_buffer.clone(),
        )
        .await?;
    println!("✅ Session created: {}\n", session_id);

    // 3. Установить breakpoint
    println!("🔖 Setting breakpoint...");
    manager.with_session(&session_id, |session| {
        Box::pin(async move {
            session.dap_client.set_breakpoints(
                r"C:\1CProject\bsl-gradual-types-milestone-4.4\mcp-debug-server\examples\test_program.rs",
                &[28]
            ).await?;
            println!("✅ Breakpoint set at line 28\n");
            Ok(())
        })
    }).await?;

    // 4. Launch программы
    println!("🚀 Launching program...");
    manager
        .with_session(&session_id, |session| {
            Box::pin(async move {
                let binary = session.binary_path.clone();

                println!("  ▶ Sending launch request...");
                session.dap_client.launch(&binary, None).await?;
                println!("  ✅ Launch response received");

                println!("  ▶ Sending configurationDone...");
                session.dap_client.configuration_done().await?;
                println!("  ✅ ConfigurationDone response received\n");

                Ok(())
            })
        })
        .await?;

    // 5. Подождать немного для событий
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 6. Проверить события
    println!("📨 Polling events...");
    let events = event_buffer.poll_events(&session_id.as_str().to_string());
    println!("  📊 Received {} events:\n", events.len());

    for (i, event) in events.iter().enumerate() {
        println!("  Event #{}: {}", i + 1, event.event);
        if let Some(body) = &event.body {
            println!("    Body: {}", serde_json::to_string_pretty(body)?);
        }
    }

    // 7. Terminate
    println!("\n🛑 Terminating session...");
    manager.terminate_session(&session_id).await?;
    println!("✅ Session terminated");

    println!("\n🎉 Test completed!");
    Ok(())
}
