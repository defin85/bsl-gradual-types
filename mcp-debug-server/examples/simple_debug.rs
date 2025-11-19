//! # Simple Debug Example
//!
//! Полный пример debug workflow с MCP Debug Server.
//! Демонстрирует создание сессии, breakpoints, stepping, evaluation.
//!
//! ## Запуск
//! ```bash
//! cargo run --example simple_debug
//! ```
//!
//! ## Что демонстрируется
//!
//! Этот пример показывает **концептуально**, как работает MCP Debug Server API.
//! Для реального использования нужен DAP adapter (CodeLLDB/GDB).
//!
//! ## Примечание
//!
//! Mock DAP Server находится в `tests/support/` и не доступен для examples.
//! Поэтому этот пример демонстрирует API **без реального запуска debugger**.
//! В комментариях показан полный workflow для реального использования.

use mcp_debug_server::session::SessionManager;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Инициализация логирования (опционально)
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== MCP Debug Server: Simple Debug Example ===\n");

    // ============================================================================
    // ЧАСТЬ 1: ДЕМОНСТРАЦИЯ API СТРУКТУРЫ (БЕЗ РЕАЛЬНОГО DEBUGGER)
    // ============================================================================

    demonstrate_debug_workflow();

    // ============================================================================
    // ЧАСТЬ 2: РЕАЛЬНЫЙ КОД (ТРЕБУЕТ DAP ADAPTER)
    // ============================================================================

    println!("\n=== Typical Real-World Usage ===\n");

    // Шаг 1: Создание SessionManager
    println!("[1/11] Creating SessionManager...");
    let _manager = Arc::new(SessionManager::new());
    println!("✓ SessionManager created\n");

    // Шаги 2-11 требуют реального DAP adapter
    // Закомментированы, т.к. требуют CodeLLDB/GDB в PATH

    /*
    // Шаг 2: Создание EventBuffer (для обработки DAP events)
    println!("[2/11] Creating EventBuffer...");
    use mcp_debug_server::dap::EventBuffer;
    let event_buffer = EventBuffer::new();
    println!("✓ EventBuffer created\n");

    // Шаг 3: Создание debug session
    println!("[3/11] Creating debug session...");
    let session_id = manager.create_session(
        "./target/debug/test_program".to_string(),  // binary path
        "codelldb".to_string(),                      // adapter type
        event_buffer.clone(),
    ).await?;
    println!("✓ Session created: {}\n", session_id);

    // Шаг 4: Установка breakpoint
    println!("[4/11] Setting breakpoint at test_program.rs:14...");
    manager.with_session(&session_id, |session| {
        Box::pin(async move {
            session.dap_client.set_breakpoints(
                "test_program.rs",
                &[14]  // line number
            ).await?;
            println!("✓ Breakpoint set\n");
            Ok(())
        })
    }).await?;

    // Шаг 5: Launch программы
    println!("[5/11] Launching program...");
    manager.with_session(&session_id, |session| {
        Box::pin(async move {
            session.dap_client.launch(
                "test_program",  // program name
                None             // no args
            ).await?;
            session.set_state(SessionState::Running)?;
            println!("✓ Program launched\n");
            Ok(())
        })
    }).await?;

    // Шаг 6: Ожидание stopped event
    println!("[6/11] Waiting for stopped event...");
    // Polling events из EventBuffer
    let events = event_buffer.poll_events(&session_id.as_str().to_string());
    for event in &events {
        if event.event == "stopped" {
            println!("✓ Stopped at breakpoint (reason: {:?})\n", event.body.get("reason"));

            // Обновить состояние
            manager.with_session(&session_id, |session| {
                Box::pin(async move {
                    session.set_state(SessionState::Stopped)?;
                    Ok(())
                })
            }).await?;
            break;
        }
    }

    // Шаг 7: Получение backtrace (stack trace)
    println!("[7/11] Getting stack trace...");
    manager.with_session(&session_id, |session| {
        Box::pin(async move {
            let thread_id = session.current_thread_id.lock().await;
            if let Some(tid) = *thread_id {
                let trace = session.dap_client.stack_trace(tid).await?;
                println!("✓ Stack trace:");
                for (i, frame) in trace.iter().enumerate() {
                    println!("  #{} {} at {}:{}",
                        i, frame.name, frame.source.as_ref().unwrap().path, frame.line);
                }
                println!();
            }
            Ok(())
        })
    }).await?;

    // Шаг 8: Вычисление expression
    println!("[8/11] Evaluating 'counter'...");
    manager.with_session(&session_id, |session| {
        Box::pin(async move {
            let result = session.dap_client.evaluate(
                "counter",  // expression
                None        // frame_id (use current)
            ).await?;
            println!("✓ counter = {:?}\n", result);
            Ok(())
        })
    }).await?;

    // Шаг 9: Step over (next line)
    println!("[9/11] Stepping to next line...");
    manager.with_session(&session_id, |session| {
        Box::pin(async move {
            let thread_id = session.current_thread_id.lock().await;
            if let Some(tid) = *thread_id {
                session.dap_client.next(tid).await?;
                println!("✓ Stepped to next line\n");
            }
            Ok(())
        })
    }).await?;

    // Шаг 10: Continue execution
    println!("[10/11] Continuing execution...");
    manager.with_session(&session_id, |session| {
        Box::pin(async move {
            let thread_id = session.current_thread_id.lock().await;
            if let Some(tid) = *thread_id {
                session.dap_client.continue_execution(tid).await?;
                session.set_state(SessionState::Running)?;
                println!("✓ Execution continued\n");
            }
            Ok(())
        })
    }).await?;

    // Шаг 11: Завершение сессии
    println!("[11/11] Terminating session...");
    manager.terminate_session(&session_id).await?;
    println!("✓ Session terminated\n");
    */

    println!("\n=== Example completed successfully ===");
    println!("\nNote: This is a demonstration of the API structure.");
    println!("For real debugging, configure Claude Desktop with MCP Debug Server.");
    println!("See README.md for configuration instructions.");

    Ok(())
}

/// Демонстрирует полный debug workflow концептуально
fn demonstrate_debug_workflow() {
    println!("=== Typical Debug Workflow (11 steps) ===\n");

    println!("[1/11] Create SessionManager:");
    println!("   let manager = SessionManager::new();");
    println!();

    println!("[2/11] Create EventBuffer (для обработки DAP events):");
    println!("   let event_buffer = EventBuffer::new();");
    println!();

    println!("[3/11] Create Debug Session:");
    println!("   let session_id = manager.create_session(");
    println!("       \"./target/debug/my_app\",  // binary path");
    println!("       \"codelldb\",                // DAP adapter type");
    println!("       event_buffer.clone(),");
    println!("   ).await?;");
    println!();

    println!("[4/11] Set Breakpoint:");
    println!("   manager.with_session(&session_id, |session| async {{");
    println!("       session.dap_client.set_breakpoints(\"main.rs\", &[42]).await");
    println!("   }}).await?;");
    println!();

    println!("[5/11] Launch Program:");
    println!("   manager.with_session(&session_id, |session| async {{");
    println!("       session.dap_client.launch(\"my_app\", None).await?;");
    println!("       session.set_state(SessionState::Running)");
    println!("   }}).await?;");
    println!();

    println!("[6/11] Wait for Stopped Event:");
    println!("   let events = event_buffer.poll_events(&session_id.as_str());");
    println!("   for event in events {{");
    println!("       if event.event == \"stopped\" {{");
    println!("           println!(\"Stopped at breakpoint\");");
    println!("           session.set_state(SessionState::Stopped)?;");
    println!("       }}");
    println!("   }}");
    println!();

    println!("[7/11] Get Stack Trace:");
    println!("   let trace = session.dap_client.stack_trace(thread_id).await?;");
    println!("   for frame in trace {{");
    println!("       println!(\"{{}} at {{}}:{{}}\", frame.name, frame.source.path, frame.line);");
    println!("   }}");
    println!();

    println!("[8/11] Evaluate Expression:");
    println!("   let result = session.dap_client.evaluate(\"my_var + 42\", None).await?;");
    println!("   println!(\"Result: {{}}\", result);");
    println!();

    println!("[9/11] Step to Next Line:");
    println!("   session.dap_client.next(thread_id).await?;");
    println!();

    println!("[10/11] Continue Execution:");
    println!("   session.dap_client.continue_execution(thread_id).await?;");
    println!("   session.set_state(SessionState::Running)?;");
    println!();

    println!("[11/11] Terminate Session:");
    println!("   manager.terminate_session(&session_id).await?;");
    println!();

    println!("=== End of Workflow ===\n");
}
